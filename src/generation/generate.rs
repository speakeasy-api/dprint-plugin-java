use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use crate::configuration::Configuration;

use super::context::FormattingContext;
use super::helpers;

/// Generate dprint PrintItems IR from a tree-sitter parse tree.
///
/// This is the core of the formatter: it walks the CST produced by
/// tree-sitter-java and emits formatting instructions that the
/// dprint-core printer resolves into final output.
///
/// The dispatcher walks the tree and delegates to specific handlers
/// based on node kind. Unhandled nodes fall back to emitting their
/// source text unchanged, allowing incremental implementation of
/// formatting rules.
pub fn generate(source: &str, tree: &tree_sitter::Tree, config: &Configuration) -> PrintItems {
    let mut context = FormattingContext::new(source, config);
    let root = tree.root_node();
    gen_node(root, &mut context)
}

/// Generate PrintItems for a tree-sitter node.
///
/// This is the main dispatcher that routes nodes to specific handlers
/// based on their kind. The match arms will be populated incrementally
/// as formatting rules are implemented.
///
/// Currently falls back to raw text for all node types, maintaining
/// pass-through behavior until specific handlers are implemented.
fn gen_node<'a>(node: tree_sitter::Node<'a>, context: &mut FormattingContext<'a>) -> PrintItems {
    match node.kind() {
        "program" => gen_program(node, context),
        // Future handlers will be added here as formatting is implemented
        // "class_declaration" => gen_class_declaration(node, context),
        // "method_declaration" => gen_method_declaration(node, context),
        // etc.
        _ => helpers::gen_node_text(node, context.source),
    }
}

/// Generate a program node (the root of the parse tree).
///
/// A program consists of zero or more top-level declarations (classes,
/// interfaces, imports, package statement, etc.). We iterate through
/// child nodes and format each one.
fn gen_program<'a>(node: tree_sitter::Node<'a>, context: &mut FormattingContext<'a>) -> PrintItems {
    let mut items = PrintItems::new();

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    for (i, child) in children.iter().enumerate() {
        // Skip pure whitespace/comment nodes for now - they're embedded in the source
        if child.is_extra() {
            continue;
        }

        items.extend(gen_node(*child, context));

        // Add newline between top-level declarations
        if i < children.len() - 1 {
            // Check if next non-extra child exists
            if children[i + 1..].iter().any(|c| !c.is_extra()) {
                items.extend(helpers::gen_newline());
            }
        }
    }

    // Handle any trailing content after the last child (e.g., final newline)
    if let Some(last_child) = children.last() {
        let last_end = last_child.end_byte();
        let program_end = node.end_byte();

        if last_end < program_end {
            // There's content after the last child - emit it
            let trailing = &context.source[last_end..program_end];
            // Only emit if it's a newline (common case)
            if trailing == "\n" || trailing == "\r\n" {
                items.extend(helpers::gen_newline());
            } else {
                // For other trailing content, emit as-is
                for line in trailing.split('\n') {
                    if !line.is_empty() {
                        let line = line.strip_suffix('\r').unwrap_or(line);
                        items.push_string(line.to_string());
                    }
                    items.push_signal(Signal::NewLine);
                }
            }
        }
    }

    items
}
