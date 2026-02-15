use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use crate::configuration::Configuration;

/// Generate dprint PrintItems IR from a tree-sitter parse tree.
///
/// This is the core of the formatter: it walks the CST produced by
/// tree-sitter-java and emits formatting instructions that the
/// dprint-core printer resolves into final output.
///
/// Currently this is a minimal pass-through implementation that
/// reproduces the source text unchanged. The formatting rules
/// (palantir-style indentation, wrapping, lambda inlining, etc.)
/// will be implemented incrementally.
pub fn generate(source: &str, tree: &tree_sitter::Tree, _config: &Configuration) -> PrintItems {
    let mut items = PrintItems::new();

    // Phase 0: pass-through — emit the source text as-is, line by line.
    // The dprint printer requires newlines to be sent as PrintItem::NewLine
    // rather than embedded in string content.
    let root = tree.root_node();
    let text = &source[root.start_byte()..root.end_byte()];

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
