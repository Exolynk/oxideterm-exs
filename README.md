# ExS Language Support for OxideTerm

Syntax highlighting for **ExS (Exolynk Xtension Script)** `.exs` files in the
native editors of [OxideTerm](https://github.com/AnalyseDeCircuit/oxideterm) 2.x
(IDE, SFTP editor and file previews).

[ExS](https://github.com/Exolynk/exs) is an open-source (MIT) extension language for
applications: scripts compile to WebAssembly and run in a sandbox on a server or in the
browser, reaching the host only through registered host functions. It is dynamically
typed with optional run-time contracts, uses `Error` values instead of exceptions, and
runs concurrent tasks with `par`. Language documentation:
<https://exolynk.github.io/exs/>.

Highlighted: keywords, strings (plain, byte, formatted `f"…{expr}…"`, raw `r#"…"#`,
dedented `d#`/`fd#`) with escapes and interpolations, numbers, comments, function and
method calls, qualified paths (`log::info`, `Color::Red`), type contracts
(`-> Int | None`), parameters, object keys and fields.

OxideTerm 2.x highlights code with Tree-sitter. This repository contains an ExS
Tree-sitter grammar, compiled to WebAssembly, and the highlight queries, packaged as
an OxideTerm language plugin (`runtime.kind: "language"`).

## Layout

| Path | Purpose |
| --- | --- |
| `tree-sitter-exs/grammar.js` | ExS grammar, mirrors the token rules of `exs-compiler`'s lexer |
| `tree-sitter-exs/src/scanner.c` | External scanner for hash-delimited raw strings (`r#"…"#`, `d#`, `f#`, `fd#`) |
| `tree-sitter-exs/queries/highlights.scm` | Highlight queries (first capture wins in OxideTerm, so specific rules come first) |
| `tree-sitter-exs/test/corpus/` | Tree-sitter parser tests |
| `plugin/` | Manifest template, NOTICE and the highlight sample OxideTerm's checker uses |
| `scripts/build.sh` | Builds `dist/com.exolynk.language.exs/` and the release ZIP |
| `verify/` | Checks the built plugin like the host does and compares it with the ExS compiler's highlighter |

The grammar is intentionally lexical (a flat token stream with small structures for
calls, paths, definitions, parameters and string interpolation), so half-written code
in an editor never breaks highlighting.

## Build

Requirements: Tree-sitter CLI **0.27.0** and `python3`. Node.js isn't required;
`grammar.js` is evaluated with the CLI's built-in QuickJS runtime.

```sh
cargo install tree-sitter-cli --version 0.27.0 --locked
scripts/build.sh            # or: TREE_SITTER=/path/to/tree-sitter scripts/build.sh
```

This regenerates the parser, runs the corpus tests, compiles `parser.wasm`, writes
`plugin.json` with the `parserSha256`/`highlightsSha256` digests, and creates
`dist/com.exolynk.language.exs-<version>.zip` (with `plugin.json` at the archive root).
It prints the ZIP's size and SHA-256 for the marketplace submission.

## Verify

```sh
cargo run --release --manifest-path verify/Cargo.toml -- \
  dist/com.exolynk.language.exs path/to/*.exs
```

The tool checks the manifest digests, compiles the highlight query, verifies that the
expected highlights in `plugin/sample.json` are visible (same rule as OxideTerm's
`check_language_plugin`), and reports how many bytes of the given files get the same
category as `exs_compiler::highlight`, which the Exolynk platform IDE uses. It links the
grammar natively from the same generated sources as `parser.wasm`.

Differences from the platform IDE that are intentional:

- Code inside `f"{…}"` interpolations is highlighted as code, not as part of the string.
- `Suite::new(` vs. `Color::Rgb(` need type information; both are shown as function calls.

## Install locally

Unpack the ZIP (or copy `dist/com.exolynk.language.exs/`) into OxideTerm's plugin
directory, so `plugin.json` ends up at:

| OS | Path |
| --- | --- |
| Linux / macOS | `~/.oxideterm/plugins/com.exolynk.language.exs/plugin.json` |
| Windows | `%APPDATA%\OxideTerm\plugins\com.exolynk.language.exs\plugin.json` |

Portable installs and custom data directories use `<data-dir>/plugins/` instead.
Then open **Plugin Manager**, reload or enable *ExS Language Support*, and open a
`.exs` file. Plugin languages take precedence over OxideTerm's legacy `.exs` → Elixir
association.

## Release

1. Bump `version` in `plugin/plugin.json` and run `scripts/build.sh`.
2. Create the immutable GitHub release `v<version>` and attach the ZIP.
3. Submit a *Plugin listing request* at
   <https://github.com/AnalyseDeCircuit/oxideterm-plugins/issues/new?template=plugin-submission.yml>
   with the release URL, `target: any`, the size and SHA-256 printed by the build.

## License

MIT, Copyright (c) 2026 Exolynk. See `LICENSE` and `plugin/NOTICE`.
