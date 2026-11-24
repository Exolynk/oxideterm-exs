//! Verifies a built ExS language plugin the way OxideTerm 2.x loads it, then
//! compares its painted colors with the ExS compiler's reference highlighter.
//!
//! Usage: exs-oxideterm-verify <plugin-dir> [file.exs ...]
//!
//! The scope mapping and the "first capture covering a byte wins" rule follow
//! OxideTerm's `oxideterm-editor-syntax` (highlight.rs, check_language_plugin.rs).

use std::{collections::BTreeMap, fs, path::PathBuf, process::ExitCode};

use sha2::{Digest, Sha256};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

unsafe extern "C" {
    fn tree_sitter_exs() -> *const ();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Scope {
    Attribute,
    Comment,
    Constant,
    Function,
    Keyword,
    Namespace,
    Number,
    Operator,
    Property,
    Punctuation,
    String,
    Type,
    Variable,
}

/// Mirrors `scope_for_capture` in OxideTerm's highlight.rs.
fn scope_for_capture(capture: &str) -> Option<Scope> {
    match capture {
        "tag.attribute" => return Some(Scope::Attribute),
        "tag.delimiter" => return Some(Scope::Punctuation),
        _ => {}
    }
    Some(match capture.split('.').next().unwrap_or(capture) {
        "attribute" => Scope::Attribute,
        "comment" => Scope::Comment,
        "constant" | "boolean" => Scope::Constant,
        "function" => Scope::Function,
        "keyword" => Scope::Keyword,
        "module" | "namespace" => Scope::Namespace,
        "number" => Scope::Number,
        "operator" => Scope::Operator,
        "property" | "field" => Scope::Property,
        "punctuation" => Scope::Punctuation,
        "string" | "character" => Scope::String,
        "type" | "constructor" | "tag" => Scope::Type,
        "variable" | "parameter" => Scope::Variable,
        _ => return None,
    })
}

struct Span {
    start: usize,
    end: usize,
    scope: Scope,
}

fn highlight_spans(query: &Query, tree: &Tree, source: &str) -> Vec<Span> {
    let mut cursor = QueryCursor::new();
    let mut captures = cursor.captures(query, tree.root_node(), source.as_bytes());
    let names = query.capture_names();
    let mut spans = Vec::new();
    while let Some((m, index)) = captures.next() {
        let capture = m.captures()[*index];
        if let Some(scope) = scope_for_capture(names[capture.index as usize]) {
            let range = capture.node.byte_range();
            if range.start < range.end {
                spans.push(Span { start: range.start, end: range.end, scope });
            }
        }
    }
    // Stable sort like `normalize_highlight_spans`.
    spans.sort_by(|a, b| a.start.cmp(&b.start).then(a.end.cmp(&b.end)));
    spans
}

/// The scope a native OxideTerm editor paints for each byte.
fn painted(spans: &[Span], len: usize) -> Vec<Option<Scope>> {
    let mut out = vec![None; len];
    for span in spans {
        for slot in &mut out[span.start..span.end.min(len)] {
            if slot.is_none() {
                *slot = Some(span.scope);
            }
        }
    }
    out
}

/// Maps the ExS compiler's categories onto OxideTerm's palette.
fn reference_scope(kind: exs_compiler::HighlightKind) -> Scope {
    use exs_compiler::HighlightKind as K;
    match kind {
        K::Keyword => Scope::Keyword,
        K::String => Scope::String,
        K::Number => Scope::Number,
        K::Operator => Scope::Operator,
        K::Punctuation => Scope::Punctuation,
        K::Type => Scope::Type,
        K::Function | K::Method => Scope::Function,
        K::Field => Scope::Property,
        K::Variant => Scope::Constant,
        K::Binding | K::Variable => Scope::Variable,
        K::Comment => Scope::Comment,
        _ => Scope::Attribute,
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(args.next().ok_or("usage: exs-oxideterm-verify <plugin-dir> [file.exs ...]")?);
    let files: Vec<PathBuf> = args.map(PathBuf::from).collect();

    // 1. Manifest and digests, as the host verifies them before loading.
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(dir.join("plugin.json"))?)?;
    let language_def = &manifest["contributes"]["language"];
    let entry = manifest["runtime"]["entry"].as_str().ok_or("missing runtime.entry")?;
    if manifest["runtime"]["kind"] != "language" {
        return Err("runtime.kind must be \"language\"".into());
    }
    let wasm = fs::read(dir.join(entry))?;
    let highlights_file = language_def["highlights"].as_str().ok_or("missing highlights")?;
    let highlights = fs::read_to_string(dir.join(highlights_file))?;
    if sha256(&wasm) != language_def["parserSha256"] || sha256(highlights.as_bytes()) != language_def["highlightsSha256"] {
        return Err("checksum mismatch".into());
    }
    if language_def["grammarName"].as_str().unwrap_or("exs") != "exs" {
        return Err("grammarName must match the exported tree_sitter_exs symbol".into());
    }

    // 2. Compile the shipped query against the grammar like the host does.
    let language = Language::new(unsafe { tree_sitter_language::LanguageFn::from_raw(tree_sitter_exs) });
    let query = Query::new(&language, &highlights)?;
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    println!("Loaded {} (ABI {}), {} query patterns", manifest["id"], language.abi_version(), query.pattern_count());

    // 3. The sample expectations, checked like `check_language_plugin`.
    let sample: serde_json::Value = serde_json::from_slice(&fs::read(dir.join("sample.json"))?)?;
    let source = sample["source"].as_str().ok_or("missing sample source")?;
    let tree = parser.parse(source, None).ok_or("parse failed")?;
    if tree.root_node().has_error() {
        return Err("grammar rejected its source sample".into());
    }
    let spans = highlight_spans(&query, &tree, source);
    let mut ok = true;
    for expected in sample["highlight"].as_array().ok_or("missing highlight list")? {
        let text = expected["text"].as_str().unwrap();
        let scope = expected["scope"].as_str().unwrap();
        let visible = spans.iter().any(|span| {
            format!("{:?}", span.scope) == scope
                && &source[span.start..span.end] == text
                && spans
                    .iter()
                    .find(|v| v.start <= span.start && span.start < v.end)
                    .is_some_and(|v| v.scope == span.scope && v.end >= span.end)
        });
        if !visible {
            println!("  FAIL sample: missing visible {scope} capture for {text:?}");
            ok = false;
        }
    }
    if ok {
        println!("Sample: all expected highlights visible");
    }

    // 4. Compare painted colors with the ExS compiler on real files.
    if files.is_empty() {
        return Ok(ok);
    }
    let mut confusion: BTreeMap<(Scope, Option<Scope>), usize> = BTreeMap::new();
    let mut examples: BTreeMap<(Scope, Option<Scope>), Vec<String>> = BTreeMap::new();
    let (mut total, mut agree, mut errors) = (0usize, 0usize, 0usize);
    for path in &files {
        let text = fs::read_to_string(path)?;
        let tree = parser.parse(&text, None).ok_or("parse failed")?;
        if tree.root_node().has_error() {
            errors += 1;
            println!("  parse error in {}", path.display());
        }
        let ours = painted(&highlight_spans(&query, &tree, &text), text.len());
        // ExS spans nest (a whole path, then its `::`); the innermost one is visible.
        let mut reference = exs_compiler::highlight(exs_compiler::Source::new(path.display().to_string(), text.as_str()));
        reference.sort_by_key(|span| std::cmp::Reverse(span.end - span.start));
        let mut expected_at: Vec<Option<Scope>> = vec![None; text.len()];
        for span in &reference {
            for slot in &mut expected_at[span.start..span.end.min(text.len())] {
                *slot = Some(reference_scope(span.kind));
            }
        }
        let mut i = 0;
        while i < text.len() {
            let Some(expected) = expected_at[i] else {
                i += 1;
                continue;
            };
            // Group a run of equal (expected, ours) bytes for a readable example.
            let start = i;
            while i < text.len() && expected_at[i] == Some(expected) && ours[i] == ours[start] {
                i += 1;
            }
            let bytes = text[start..i].bytes().filter(|b| !b.is_ascii_whitespace()).count();
            total += bytes;
            if ours[start] == Some(expected) {
                agree += bytes;
                continue;
            }
            let key = (expected, ours[start]);
            *confusion.entry(key).or_default() += bytes;
            let list = examples.entry(key).or_default();
            if list.len() < 3 && bytes > 0 {
                let line_start = text[..start].rfind('\n').map_or(0, |n| n + 1);
                let line_end = text[i..].find('\n').map_or(text.len(), |n| i + n);
                let line: String = text[line_start..line_end].trim().chars().take(60).collect();
                list.push(format!("{:?} in {line:?}", &text[start..i]));
            }
        }
    }
    println!(
        "Corpus: {} files, {} with parse errors, {:.2}% of {} highlighted bytes agree with the ExS compiler",
        files.len(),
        errors,
        100.0 * agree as f64 / total.max(1) as f64,
        total
    );
    let mut rows: Vec<_> = confusion.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    for ((expected, got), bytes) in rows.iter().take(25) {
        println!(
            "  {bytes:>8} bytes  ExS {expected:?} -> OxideTerm {got:?}\n             e.g. {}",
            examples[&(*expected, *got)].join("\n                  ")
        );
    }
    Ok(ok && errors == 0)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
