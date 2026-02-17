use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

/// Extension trait for `PrintItems` that reduces boilerplate.
///
/// Replaces verbose patterns like `items.push_string("x".to_string())`,
/// `items.push_signal(Signal::NewLine)`, and `items.extend(gen_space())`
/// with concise methods: `items.push_str("x")`, `items.newline()`, `items.space()`.
pub trait PrintItemsExt {
    fn push_str(&mut self, s: &str);
    fn space(&mut self);
    fn newline(&mut self);
    fn start_indent(&mut self);
    fn finish_indent(&mut self);
}

impl PrintItemsExt for PrintItems {
    #[inline]
    fn push_str(&mut self, s: &str) {
        self.push_string(s.to_string());
    }

    #[inline]
    fn space(&mut self) {
        self.push_string(" ".to_string());
    }

    #[inline]
    fn newline(&mut self) {
        self.push_signal(Signal::NewLine);
    }

    #[inline]
    fn start_indent(&mut self) {
        self.push_signal(Signal::StartIndent);
    }

    #[inline]
    fn finish_indent(&mut self) {
        self.push_signal(Signal::FinishIndent);
    }
}

/// Check if a tree-sitter node kind is a Java type node.
///
/// Used to deduplicate the repeated type-kind match patterns
/// that appear in method declarations, field declarations,
/// formal parameters, etc.
pub fn is_type_node(kind: &str) -> bool {
    matches!(
        kind,
        "void_type"
            | "integral_type"
            | "floating_point_type"
            | "boolean_type"
            | "type_identifier"
            | "scoped_type_identifier"
            | "generic_type"
            | "array_type"
    )
}

/// Estimate the "flat" width of a code fragment as if formatted on one line.
///
/// Collapses newlines and runs of whitespace into single spaces, then
/// returns the length. Avoids `String` allocation.
pub fn collapse_whitespace_len(s: &str) -> usize {
    let s = s.trim();
    let mut len = 0;
    let mut prev_was_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                len += 1;
                prev_was_space = true;
            }
        } else {
            len += 1;
            prev_was_space = false;
        }
    }
    len
}

/// Extract the source text for a tree-sitter node.
///
/// Properly handles newlines by emitting them as `Signal::NewLine`
/// rather than embedding them in strings, which is required by dprint-core.
///
/// For multiline text, leading whitespace on non-first lines is stripped
/// since `Signal::NewLine` already provides the correct indentation from
/// dprint-core's indent stack. Keeping original whitespace would cause
/// double-indentation that compounds on each formatting pass.
pub fn gen_node_text(node: tree_sitter::Node, source: &str) -> PrintItems {
    let text = &source[node.start_byte()..node.end_byte()];
    let mut items = PrintItems::new();

    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            items.newline();
        }

        let line = line.strip_suffix('\r').unwrap_or(line);
        let content = if i > 0 { line.trim_start() } else { line };
        if !content.is_empty() {
            items.push_str(content);
        }
    }

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

    #[test]
    fn test_collapse_whitespace_len() {
        assert_eq!(collapse_whitespace_len("  hello   world  "), 11);
        assert_eq!(collapse_whitespace_len("a\n    b\n    c"), 5);
        assert_eq!(collapse_whitespace_len(""), 0);
        assert_eq!(collapse_whitespace_len("foo bar"), 7);
        assert_eq!(collapse_whitespace_len("single"), 6);
    }

    #[test]
    fn test_is_type_node() {
        assert!(is_type_node("void_type"));
        assert!(is_type_node("type_identifier"));
        assert!(is_type_node("scoped_type_identifier"));
        assert!(is_type_node("generic_type"));
        assert!(is_type_node("array_type"));
        assert!(!is_type_node("identifier"));
        assert!(!is_type_node("block"));
    }
}
