use dprint_core::formatting::*;

use super::context::FormattingContext;

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

/// Generate a separated list of items (e.g., comma-separated parameters).
///
/// This wraps items when they don't fit on one line and handles the separator
/// (typically a comma) with optional space or newline after it.
pub fn gen_separated_list<'a, F>(
    children: impl Iterator<Item = tree_sitter::Node<'a>>,
    separator: &str,
    context: &mut FormattingContext<'a>,
    mut gen_item: F,
) -> PrintItems
where
    F: FnMut(tree_sitter::Node<'a>, &mut FormattingContext<'a>) -> PrintItems,
{
    let mut items = PrintItems::new();
    let children_vec: Vec<_> = children.collect();

    for (i, child) in children_vec.iter().enumerate() {
        items.extend(gen_item(*child, context));

        if i < children_vec.len() - 1 {
            items.push_string(separator.to_string());
            items.push_signal(Signal::SpaceOrNewLine);
        }
    }

    items
}

/// Generate a block with opening brace, body items, and closing brace.
///
/// This handles the common pattern of `{ body }` with proper indentation.
/// The body items are indented and placed on new lines if they don't fit.
///
/// Example formatting:
/// ```java
/// {
///     statement1;
///     statement2;
/// }
/// ```
pub fn gen_block<'a>(
    open_brace: &str,
    body_items: PrintItems,
    close_brace: &str,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();

    // Opening brace
    items.push_string(open_brace.to_string());

    // Check if body is empty
    if body_items.is_empty() {
        // Empty block: just close it
        items.push_string(close_brace.to_string());
        return items;
    }

    // Non-empty block: newline, indent, body, dedent, newline, close
    items.push_signal(Signal::NewLine);

    context.indent();
    items.push_signal(Signal::StartIndent);
    items.extend(body_items);
    items.push_signal(Signal::FinishIndent);
    context.dedent();

    items.push_signal(Signal::NewLine);
    items.push_string(close_brace.to_string());

    items
}

/// Generate items with forced indentation increase/decrease.
///
/// This is useful for wrapping a section of code that should be indented
/// relative to the surrounding context.
pub fn gen_with_indent<'a>(
    inner_items: PrintItems,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();

    context.indent();
    items.push_signal(Signal::StartIndent);
    items.extend(inner_items);
    items.push_signal(Signal::FinishIndent);
    context.dedent();

    items
}

/// Generate a space-or-newline separator that can break at this point.
pub fn gen_space_or_newline() -> PrintItems {
    let mut items = PrintItems::new();
    items.push_signal(Signal::SpaceOrNewLine);
    items
}

/// Generate a mandatory newline.
pub fn gen_newline() -> PrintItems {
    let mut items = PrintItems::new();
    items.push_signal(Signal::NewLine);
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
    use crate::configuration::Configuration;
    use dprint_core::configuration::NewLineKind;

    fn test_config() -> Configuration {
        Configuration {
            line_width: 120,
            indent_width: 4,
            use_tabs: false,
            new_line_kind: NewLineKind::LineFeed,
            format_javadoc: false,
            method_chain_threshold: 80,
            inline_lambdas: true,
        }
    }

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
    fn test_gen_block_empty() {
        let config = test_config();
        let mut context = FormattingContext::new("", &config);

        let items = gen_block("{", PrintItems::new(), "}", &mut context);
        assert!(!items.is_empty());
    }

    #[test]
    fn test_gen_block_with_content() {
        let config = test_config();
        let mut context = FormattingContext::new("", &config);

        let mut body = PrintItems::new();
        body.push_string("statement;".to_string());

        let items = gen_block("{", body, "}", &mut context);
        assert!(!items.is_empty());
    }
}
