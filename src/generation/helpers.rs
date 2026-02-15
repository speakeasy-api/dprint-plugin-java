use dprint_core::formatting::*;

/// Extract the source text for a tree-sitter node.
///
/// This properly handles newlines by emitting them as Signal::NewLine
/// rather than embedding them in strings, which is required by dprint-core.
pub fn gen_node_text(node: tree_sitter::Node, source: &str) -> PrintItems {
    let text = &source[node.start_byte()..node.end_byte()];
    let mut items = PrintItems::new();

    // Split on newlines and emit each line separately
    let mut first = true;
    for line in text.split('\n') {
        if !first {
            items.push_signal(Signal::NewLine);
        }
        first = false;

        // Strip trailing \r for CRLF files
        let line = line.strip_suffix('\r').unwrap_or(line);
        if !line.is_empty() {
            items.push_string(line.to_string());
        }
    }

    items
}

/// Generate a mandatory space.
pub fn gen_space() -> PrintItems {
    let mut items = PrintItems::new();
    items.push_string(" ".to_string());
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_node_text() {
        let source = "public class Hello {}";
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = tree.root_node();

        let items = gen_node_text(root, source);
        assert!(!items.is_empty());
    }

}
