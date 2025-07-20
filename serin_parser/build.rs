fn main() {
    if std::env::var("CARGO_FEATURE_ANTLR4").is_ok() {
        println!("cargo:rerun-if-changed=grammar/SQL.g4");
        // Placeholder: invoke Antlr4 codegen via external script or build tool.
        // Keeping empty to allow build without Antlr4 installed.
    }
} 