pub fn bundled() -> String {
    include_str!("../assets/agent-instructions.md")
        .replace("{{VERSION}}", env!("CARGO_PKG_VERSION"))
}
