fn main() {
    let src = std::path::Path::new("../tree-sitter-exs/src");
    cc::Build::new()
        .include(src)
        .file(src.join("parser.c"))
        .file(src.join("scanner.c"))
        .warnings(false)
        .compile("tree-sitter-exs");
    println!("cargo:rerun-if-changed=../tree-sitter-exs/src");
}
