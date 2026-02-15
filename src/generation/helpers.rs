use dprint_core::formatting::*;

/// Extract the source text for a tree-sitter node.
///
/// This properly handles newlines by emitting them as Signal::NewLine
/// rather than embedding them in strings, which is required by dprint-core.
///
/// For multiline text, leading whitespace on non-first lines is stripped
/// since Signal::NewLine already provides the correct indentation from
/// dprint-core's indent stack. Keeping original whitespace would cause
/// double-indentation that compounds on each formatting pass.
pub fn gen_node_text(node: tree_sitter::Node, source: &str) -> PrintItems {
    let text = &source[node.start_byte()..node.end_byte()];
    let mut items = PrintItems::new();

    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            items.push_signal(Signal::NewLine);
        }

        // Strip trailing \r for CRLF files
        let line = line.strip_suffix('\r').unwrap_or(line);

        // For non-first lines, strip leading whitespace since Signal::NewLine
        // already handles indentation via dprint-core's indent stack.
        let content = if i > 0 { line.trim_start() } else { line };
        if !content.is_empty() {
            items.push_string(content.to_string());
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
