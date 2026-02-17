use dprint_core::formatting::PrintItems;

use super::comments;
use super::context::FormattingContext;
use super::expressions;
use super::generate::gen_node;
use super::helpers::{PrintItemsExt, collapse_whitespace_len, gen_node_text, is_type_node};

/// Format a package declaration: `package com.example;`
pub fn gen_package_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "package" => items.push_str("package"),
            "scoped_identifier" | "identifier" => {
                items.space();
                items.extend(gen_node_text(child, context.source));
            }
            ";" => items.push_str(";"),
            _ => {}
        }
    }

    items
}

/// Format an import declaration: `import java.util.List;`
pub fn gen_import_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "import" => items.push_str("import"),
            "static" => {
                items.space();
                items.push_str("static");
            }
            "scoped_identifier" | "identifier" => {
                items.space();
                items.extend(gen_node_text(child, context.source));
            }
            "asterisk" => {
                items.push_str(".*");
            }
            ";" => items.push_str(";"),
            _ => {}
        }
    }

    items
}

/// Format a class declaration.
pub fn gen_class_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    // Pre-calculate: estimate class declaration line width to decide extends/implements wrapping.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let decl_width = estimate_class_decl_width(node, context.source);
    // +2 for trailing " {" after the class declaration
    let needs_wrapping = indent_width + decl_width + 2 > context.config.line_width as usize;

    // When both extends and implements are present, prefer to wrap only before implements.
    // Only wrap before extends if implements is not present and extends alone is too long.
    let mut cursor2 = node.walk();
    let has_superclass = node
        .children(&mut cursor2)
        .any(|c| c.kind() == "superclass");
    let has_super_interfaces = node
        .children(&mut cursor2)
        .any(|c| c.kind() == "super_interfaces");

    let wrap_extends = needs_wrapping && has_superclass && !has_super_interfaces;
    let wrap_implements = needs_wrapping && has_super_interfaces;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            "class" => {
                if need_space {
                    items.space();
                }
                items.push_str("class");
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = true;
            }
            "type_parameters" => {
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            "superclass" => {
                if wrap_extends {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    context.add_continuation_indent(2);
                    items.extend(gen_superclass(child, context));
                    context.remove_continuation_indent(2);
                    items.finish_indent();
                    items.finish_indent();
                } else {
                    items.space();
                    items.extend(gen_superclass(child, context));
                }
                need_space = true;
            }
            "super_interfaces" => {
                if wrap_implements {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    context.add_continuation_indent(2);
                    items.extend(gen_super_interfaces(child, context));
                    context.remove_continuation_indent(2);
                    items.finish_indent();
                    items.finish_indent();
                } else {
                    items.space();
                    items.extend(gen_super_interfaces(child, context));
                }
                need_space = true;
            }
            "class_body" => {
                items.space();
                items.extend(gen_class_body(child, context));
                need_space = false;
            }
            _ => {}
        }
    }

    items
}

/// Format an interface declaration.
pub fn gen_interface_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    // Pre-calculate: estimate interface declaration line width to decide extends wrapping.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let decl_width = estimate_class_decl_width(node, context.source);
    // +2 for trailing " {" after the interface declaration
    let wrap_clauses = indent_width + decl_width + 2 > context.config.line_width as usize;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            "interface" => {
                if need_space {
                    items.space();
                }
                items.push_str("interface");
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = true;
            }
            "type_parameters" => {
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            "extends_interfaces" => {
                if wrap_clauses {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    context.add_continuation_indent(2);
                    items.extend(gen_extends_interfaces(child, context));
                    context.remove_continuation_indent(2);
                    items.finish_indent();
                    items.finish_indent();
                } else {
                    items.space();
                    items.extend(gen_extends_interfaces(child, context));
                }
                need_space = true;
            }
            "interface_body" => {
                items.space();
                items.extend(gen_interface_body(child, context));
                need_space = false;
            }
            _ => {}
        }
    }

    items
}

/// Format an enum declaration.
pub fn gen_enum_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    // Pre-calculate: estimate enum declaration line width to decide implements wrapping.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let decl_width = estimate_class_decl_width(node, context.source);
    let wrap_clauses = indent_width + decl_width > context.config.line_width as usize;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            "enum" => {
                if need_space {
                    items.space();
                }
                items.push_str("enum");
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = true;
            }
            "super_interfaces" => {
                if wrap_clauses {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    items.extend(gen_super_interfaces(child, context));
                    items.finish_indent();
                    items.finish_indent();
                } else {
                    items.space();
                    items.extend(gen_super_interfaces(child, context));
                }
                need_space = true;
            }
            "enum_body" => {
                items.space();
                items.extend(gen_enum_body(child, context));
                need_space = false;
            }
            _ => {}
        }
    }

    items
}

/// Format a record declaration (Java 16+).
pub fn gen_record_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    // Pre-calculate: estimate record declaration line width to decide implements wrapping.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let decl_width = estimate_class_decl_width(node, context.source);
    let wrap_clauses = indent_width + decl_width > context.config.line_width as usize;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            "record" => {
                if need_space {
                    items.space();
                }
                items.push_str("record");
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = false;
            }
            "formal_parameters" => {
                items.extend(gen_formal_parameters(child, context));
                need_space = true;
            }
            "super_interfaces" => {
                if wrap_clauses {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    items.extend(gen_super_interfaces(child, context));
                    items.finish_indent();
                    items.finish_indent();
                } else {
                    items.space();
                    items.extend(gen_super_interfaces(child, context));
                }
                need_space = true;
            }
            "class_body" => {
                items.space();
                items.extend(gen_class_body(child, context));
                need_space = false;
            }
            _ => {}
        }
    }

    items
}

/// Format an annotation type declaration: `@interface MyAnnotation { ... }`
pub fn gen_annotation_type_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            "@interface" => {
                if need_space {
                    items.space();
                }
                items.push_str("@interface");
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = true;
            }
            "annotation_type_body" => {
                items.space();
                items.extend(gen_annotation_type_body(child, context));
                need_space = false;
            }
            _ => {}
        }
    }

    items
}

/// Format a method declaration.
///
/// Handles wrapping of the throws clause onto a continuation line when the
/// method signature would exceed `line_width`.
#[allow(clippy::too_many_lines)]
pub fn gen_method_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    // Pre-calculate: estimate method signature line width to decide throws wrapping.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let sig_width = estimate_method_sig_width(node, context.source);
    let line_width = context.config.line_width as usize;
    // +2 for the trailing " {" or ";" that follows the throws clause
    let full_too_wide = indent_width + sig_width + 2 > line_width;
    // PJF wraps throws when the line containing `) throws ... {` would exceed line_width.
    // If params fit inline, this is the full flat sig width.
    // If params are wrapped, the `)` is on the last param line (shorter).
    let wrap_throws = if full_too_wide {
        let mut c = node.walk();
        let children_vec: Vec<_> = node.children(&mut c).collect();
        // Compute width of signature WITHOUT the throws clause
        let sig_no_throws: usize = {
            let mut w = 0;
            let mut c2 = node.walk();
            for ch in node.children(&mut c2) {
                match ch.kind() {
                    "block" | "constructor_body" | ";" | "throws" => break,
                    _ => {
                        let text = &context.source[ch.start_byte()..ch.end_byte()];
                        let last_line = text.lines().last().unwrap_or(text);
                        if w > 0
                            && ch.kind() != "formal_parameters"
                            && ch.kind() != "("
                            && ch.kind() != ")"
                        {
                            w += 1; // space
                        }
                        w += last_line.trim().len();
                    }
                }
            }
            w
        };
        let params_fit_inline = indent_width + sig_no_throws <= line_width;
        if params_fit_inline {
            // Params on one line: throws wraps based on full sig width
            true
        } else {
            // Params will wrap. Check if `) throws ... {` fits on the last param line.
            let throws_width: usize = children_vec
                .iter()
                .find(|ch| ch.kind() == "throws")
                .map_or(0, |throws_node| {
                    let text =
                        &context.source[throws_node.start_byte()..throws_node.end_byte()];
                    collapse_whitespace_len(text)
                });
            if throws_width == 0 {
                false
            } else {
                let last_param_width = children_vec
                    .iter()
                    .find(|ch| ch.kind() == "formal_parameters")
                    .and_then(|params| {
                        let mut pc = params.walk();
                        params
                            .children(&mut pc)
                            .filter(|p| {
                                p.kind() == "formal_parameter"
                                    || p.kind() == "spread_parameter"
                            })
                            .last()
                            .map(|p| {
                                let text = &context.source[p.start_byte()..p.end_byte()];
                                collapse_whitespace_len(text)
                            })
                    })
                    .unwrap_or(0);
                let continuation_col =
                    indent_width + 2 * context.config.indent_width as usize;
                // Last param line: continuation + last_param + ") throws ... {"
                continuation_col + last_param_width + 2 + throws_width + 2 > line_width
            }
        }
    } else {
        false
    };

    // PJF: wrap between return type and method name when the signature is too long.
    // Example: `public CompletableFuture<VeryLongResponse>\n        methodName(params) {`
    let mut wrap_before_name = {
        let mut cursor_pre = node.walk();
        let children_pre: Vec<_> = node.children(&mut cursor_pre).collect();
        // Find the method name (identifier) position
        let name_idx = children_pre.iter().position(|c| c.kind() == "identifier");
        if let Some(idx) = name_idx {
            // Width of everything up to and including the return type
            let mut return_type_width = 0;
            for c in &children_pre[..idx] {
                let text = &context.source[c.start_byte()..c.end_byte()];
                let last_line = text.lines().last().unwrap_or(text);
                if return_type_width > 0 {
                    return_type_width += 1; // space
                }
                return_type_width += last_line.trim().len();
            }
            // Width of identifier + remaining sig (params, throws)
            let name_text =
                &context.source[children_pre[idx].start_byte()..children_pre[idx].end_byte()];
            let name_width = name_text.len();
            // Estimate params width
            let params_width: usize = children_pre
                .iter()
                .find_map(|c| {
                    if c.kind() == "formal_parameters" {
                        let text = &context.source[c.start_byte()..c.end_byte()];
                        Some(collapse_whitespace_len(text))
                    } else {
                        None
                    }
                })
                .unwrap_or(2); // "()" minimum
            // PJF wraps before method name only when return_type + name + "(" alone
            // doesn't fit (not just when the full sig with params is too long).
            // If wrapping params alone can fix it, we don't wrap the name.
            let name_line_width = indent_width + return_type_width + 1 + name_width + 1; // +1 for "("
            let continuation_col = indent_width + 2 * context.config.indent_width as usize;
            let name_at_continuation = continuation_col + name_width + params_width;
            name_line_width > line_width && name_at_continuation <= line_width
        } else {
            false
        }
    };

    let mut did_wrap_name = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            "type_parameters" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            // Return type: various type nodes
            kind if is_type_node(kind) => {
                if need_space {
                    items.space();
                }
                context.start_type_args_wrap_tracking();
                items.extend(gen_node(child, context));
                if context.finish_type_args_wrap_tracking() {
                    wrap_before_name = true;
                }
                need_space = true;
            }
            "identifier" => {
                if wrap_before_name {
                    // Wrap: put method name on continuation-indent line
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    did_wrap_name = true;
                    // Tell formal_parameters the effective prefix is just the method name
                    let name_text = &context.source[child.start_byte()..child.end_byte()];
                    context.set_override_prefix_width(Some(name_text.len()));
                } else if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = false;
            }
            "formal_parameters" => {
                items.extend(gen_formal_parameters(child, context));
                need_space = true;
            }
            "throws" => {
                if wrap_throws {
                    if !did_wrap_name {
                        items.start_indent();
                        items.start_indent();
                    }
                    items.newline();
                    items.extend(gen_throws(child, context));
                    if !did_wrap_name {
                        items.finish_indent();
                        items.finish_indent();
                    }
                } else {
                    items.space();
                    items.extend(gen_throws(child, context));
                }
                need_space = true;
            }
            "block" => {
                if did_wrap_name {
                    items.finish_indent();
                    items.finish_indent();
                }
                items.space();
                items.extend(gen_node(child, context));
                need_space = false;
                did_wrap_name = false; // consumed
            }
            ";" => {
                if did_wrap_name {
                    items.finish_indent();
                    items.finish_indent();
                    did_wrap_name = false;
                }
                items.push_str(";");
                need_space = false;
            }
            "dimensions" => {
                items.extend(gen_node_text(child, context.source));
                need_space = true;
            }
            _ => {}
        }
    }

    if did_wrap_name {
        items.finish_indent();
        items.finish_indent();
    }

    items
}

/// Estimate the width of a method signature line (modifiers + return type + name + params + throws)
/// from the source text. Only considers the "flat" width, ignoring existing line breaks.
fn estimate_method_sig_width(node: tree_sitter::Node, source: &str) -> usize {
    let mut cursor = node.walk();
    let mut width = 0;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "block" | "constructor_body" => break, // Stop at body
            ";" => {
                width += 1;
                break;
            }
            _ => {
                let text = &source[child.start_byte()..child.end_byte()];
                // Use first line only (for multiline modifiers like annotations)
                let first_line = text.lines().last().unwrap_or(text);
                if width > 0
                    && child.kind() != "formal_parameters"
                    && child.kind() != "("
                    && child.kind() != ")"
                {
                    width += 1; // space separator
                }
                width += first_line.trim().len();
            }
        }
    }

    width
}

/// Estimate the prefix width before a `formal_parameters` or `argument_list` node.
/// This is the text that appears on the same line before the opening `(`:
/// - For methods: modifiers + return type + method name
/// - For constructors: modifiers + constructor name
/// - For method invocations: receiver + method name
/// - For object creation: `new` + type name
///
/// Uses the parent-to-node text as the base measurement, then walks up
/// ancestors to account for keywords/LHS that share the same line.
pub(super) fn estimate_prefix_width(node: tree_sitter::Node, source: &str, assignment_wrapped: bool) -> usize {
    let Some(parent) = node.parent() else { return 0 };

    // Extract the text from the start of the parent to the start of this node
    let prefix_text = &source[parent.start_byte()..node.start_byte()];

    // Only consider the last line to handle multiline modifiers/annotations
    let last_line = prefix_text.lines().last().unwrap_or(prefix_text);
    let mut width = last_line.trim_start().len();

    // Walk up ancestors to accumulate prefix from keywords/LHS that share the line.
    // Stop when we hit a node that may introduce a line break (e.g., variable_declarator
    // wraps at `=`, method_declaration can wrap return type from name).
    let mut prev = parent;
    let mut ancestor = parent.parent();
    let parent_start_row = parent.start_position().row;
    while let Some(anc) = ancestor {
        // Only add prefix from ancestors that start on the same source line
        if anc.start_position().row != parent_start_row {
            break;
        }
        match anc.kind() {
            "return_statement" => {
                width += 7; // "return "
                break;
            }
            "throw_statement" => {
                width += 6; // "throw "
                break;
            }
            "assignment_expression" => {
                // If the assignment is being wrapped at '=', the RHS starts on a new
                // line at continuation indent — don't count LHS as prefix width.
                if !assignment_wrapped {
                    let lhs_text = &source[anc.start_byte()..prev.start_byte()];
                    let lhs_last_line = lhs_text.lines().last().unwrap_or(lhs_text);
                    width += lhs_last_line.trim_start().len();
                }
                break;
            }
            "variable_declarator" | "local_variable_declaration" | "field_declaration" => {
                // If the assignment already wrapped at '=', the RHS starts on a new
                // line at continuation indent — don't count LHS as prefix width.
                if !assignment_wrapped {
                    let lhs_text = &source[anc.start_byte()..prev.start_byte()];
                    let lhs_last_line = lhs_text.lines().last().unwrap_or(lhs_text);
                    width += lhs_last_line.trim_start().len();
                }
                // Continue walking up if there's a containing declaration
                prev = anc;
                ancestor = anc.parent();
            }
            // These are wrapping boundaries — stop walking
            "method_declaration" | "constructor_declaration" => break,
            _ => {
                prev = anc;
                ancestor = anc.parent();
            }
        }
    }

    width
}

/// Estimate the width of a class/interface/enum/record declaration line
/// (modifiers + keyword + name + `type_parameters` + extends/implements + body start)
/// from the source text. Only considers the "flat" width, ignoring existing line breaks.
fn estimate_class_decl_width(node: tree_sitter::Node, source: &str) -> usize {
    let mut cursor = node.walk();
    let mut width = 0;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "class_body" | "interface_body" | "enum_body" => break, // Stop at body
            "modifiers" => {
                let text = &source[child.start_byte()..child.end_byte()];
                // Use last line only (for multiline modifiers like annotations)
                let last_line = text.lines().last().unwrap_or(text);
                width += last_line.trim().len();
            }
            _ => {
                let text = &source[child.start_byte()..child.end_byte()];
                // Use collapsed width for all non-modifier nodes to avoid
                // instability when the source text has been wrapped from a
                // previous formatting pass.
                let flat_len = collapse_whitespace_len(text);
                if width > 0
                    && child.kind() != "formal_parameters"
                    && child.kind() != "("
                    && child.kind() != ")"
                {
                    width += 1; // space separator
                }
                width += flat_len;
            }
        }
    }

    width
}

/// Format a constructor declaration.
///
/// Handles wrapping of the throws clause onto a continuation line when the
/// constructor signature would exceed `line_width`.
pub fn gen_constructor_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    // Pre-calculate: estimate constructor signature line width to decide throws wrapping.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let sig_width = estimate_method_sig_width(node, context.source);
    let line_width = context.config.line_width as usize;
    // +2 for the trailing " {" that follows the throws clause
    let full_too_wide = indent_width + sig_width + 2 > line_width;
    let wrap_throws = if full_too_wide {
        // Check if params fit inline (without wrapping)
        let sig_no_throws: usize = {
            let mut w = 0;
            let mut c2 = node.walk();
            for ch in node.children(&mut c2) {
                match ch.kind() {
                    "block" | "constructor_body" | ";" | "throws" => break,
                    _ => {
                        let text = &context.source[ch.start_byte()..ch.end_byte()];
                        let last_line = text.lines().last().unwrap_or(text);
                        if w > 0
                            && ch.kind() != "formal_parameters"
                            && ch.kind() != "("
                            && ch.kind() != ")"
                        {
                            w += 1;
                        }
                        w += last_line.trim().len();
                    }
                }
            }
            w
        };
        if indent_width + sig_no_throws <= line_width {
            // Params fit inline: wrap throws based on full sig width
            true
        } else {
            // Params will wrap. Check last param line + throws.
            let mut c = node.walk();
            let children_vec: Vec<_> = node.children(&mut c).collect();
            let throws_width: usize = children_vec
                .iter()
                .find(|ch| ch.kind() == "throws")
                .map_or(0, |throws_node| {
                    let text =
                        &context.source[throws_node.start_byte()..throws_node.end_byte()];
                    collapse_whitespace_len(text)
                });
            if throws_width == 0 {
                false
            } else {
                let last_param_width = children_vec
                    .iter()
                    .find(|ch| ch.kind() == "formal_parameters")
                    .and_then(|params| {
                        let mut pc = params.walk();
                        params
                            .children(&mut pc)
                            .filter(|p| {
                                p.kind() == "formal_parameter"
                                    || p.kind() == "spread_parameter"
                            })
                            .last()
                            .map(|p| {
                                let text = &context.source[p.start_byte()..p.end_byte()];
                                collapse_whitespace_len(text)
                            })
                    })
                    .unwrap_or(0);
                let continuation_col = indent_width + 2 * context.config.indent_width as usize;
                continuation_col + last_param_width + 2 + throws_width + 2 > line_width
            }
        }
    } else {
        false
    };

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            "type_parameters" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = false;
            }
            "formal_parameters" => {
                items.extend(gen_formal_parameters(child, context));
                need_space = true;
            }
            "throws" => {
                if wrap_throws {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    items.extend(gen_throws(child, context));
                    items.finish_indent();
                    items.finish_indent();
                } else {
                    items.space();
                    items.extend(gen_throws(child, context));
                }
                need_space = true;
            }
            "constructor_body" => {
                items.space();
                items.extend(gen_node(child, context));
                need_space = false;
            }
            _ => {}
        }
    }

    items
}

/// Format a field declaration: `private String name;`
pub fn gen_field_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;
    let mut type_args_wrapped = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            // Type nodes
            kind if is_type_node(kind) => {
                if need_space {
                    items.space();
                }
                context.start_type_args_wrap_tracking();
                items.extend(gen_node(child, context));
                type_args_wrapped = context.finish_type_args_wrap_tracking();
                need_space = true;
            }
            "variable_declarator" => {
                if type_args_wrapped {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                    context.indent();
                    context.indent();
                    context.set_declarator_on_new_line(true);
                    items.extend(gen_variable_declarator(child, context));
                    context.set_declarator_on_new_line(false);
                    context.dedent();
                    context.dedent();
                    items.finish_indent();
                    items.finish_indent();
                    type_args_wrapped = false;
                } else {
                    if need_space {
                        items.space();
                    }
                    items.extend(gen_variable_declarator(child, context));
                }
                need_space = false;
            }
            "," => {
                items.push_str(",");
                need_space = true;
            }
            ";" => {
                items.push_str(";");
                need_space = false;
            }
            _ => {}
        }
    }

    items
}

// --- Internal helpers ---

/// JLS canonical order for Java modifiers (JLS 8.1.1, 8.3.1, 8.4.3)
const JLS_MODIFIER_ORDER: &[&str] = &[
    "public",
    "protected",
    "private",
    "abstract",
    "default",
    "static",
    "final",
    "transient",
    "volatile",
    "synchronized",
    "native",
    "strictfp",
    "sealed",
    "non-sealed",
];

/// Format modifiers (public, static, final, abstract, etc.)
///
/// Annotations are placed on their own line before keyword modifiers.
/// Keyword modifiers are reordered to JLS canonical order.
///
/// Returns (items, `ends_with_newline`) where `ends_with_newline` is true
/// if the output ends with a newline (i.e., has annotations but no keywords).
pub fn gen_modifiers<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> (PrintItems, bool) {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Separate annotations from keyword modifiers
    let annotations: Vec<_> = children
        .iter()
        .filter(|c| c.kind() == "marker_annotation" || c.kind() == "annotation")
        .collect();
    let mut keywords: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "marker_annotation" && c.kind() != "annotation")
        .collect();

    // Sort keyword modifiers by JLS canonical order
    keywords.sort_by_key(|kw| {
        let text = &context.source[kw.start_byte()..kw.end_byte()];
        JLS_MODIFIER_ORDER
            .iter()
            .position(|m| *m == text)
            .unwrap_or(usize::MAX)
    });

    // Emit annotations, each on their own line
    for ann in &annotations {
        items.extend(gen_node(**ann, context));
        // Always add newline after each annotation
        items.newline();
    }

    // Emit keyword modifiers on a single line
    let mut first = true;
    for kw in &keywords {
        if !first {
            items.space();
        }
        items.extend(gen_node_text(**kw, context.source));
        first = false;
    }

    // Return true if we ended with a newline (annotations but no keywords)
    let ends_with_newline = !annotations.is_empty() && keywords.is_empty();
    (items, ends_with_newline)
}

/// Format type parameters: `<T, U extends Comparable<U>>`
fn gen_type_parameters<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "<" => items.push_str("<"),
            ">" => items.push_str(">"),
            "," => {
                items.push_str(",");
                items.space();
            }
            _ => {
                items.extend(gen_node(child, context));
            }
        }
    }

    items
}

/// Format `extends BaseClass`
fn gen_superclass<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "extends" => items.push_str("extends"),
            _ if child.is_named() => {
                items.space();
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format `implements Interface1, Interface2`
fn gen_super_interfaces<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "implements" => {
                items.push_str("implements");
            }
            "type_list" => {
                items.space();
                items.extend(gen_type_list(child, context));
            }
            "," => {
                items.push_str(",");
            }
            _ if child.is_named() => {
                items.space();
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format `extends Interface1, Interface2` (for interfaces)
fn gen_extends_interfaces<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "extends" => {
                items.push_str("extends");
            }
            "type_list" => {
                items.space();
                items.extend(gen_type_list(child, context));
            }
            "," => {
                items.push_str(",");
            }
            _ if child.is_named() => {
                items.space();
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a type list (comma-separated types).
fn gen_type_list<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "," => {
                items.push_str(",");
                items.space();
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a class body: `{ members }`
pub fn gen_class_body<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    gen_body_with_members(node, context)
}

/// Format an interface body.
pub fn gen_interface_body<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    gen_body_with_members(node, context)
}

/// Format an annotation type body.
pub fn gen_annotation_type_body<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    gen_body_with_members(node, context)
}

/// Format an enum body: `{ CONSTANT1, CONSTANT2; methods... }`
#[allow(clippy::too_many_lines)]
fn gen_enum_body<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("{");

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Collect children excluding braces
    let members: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "{" && c.kind() != "}")
        .collect();

    if members.is_empty() {
        items.push_str("}");
        return items;
    }

    // Use dprint-core indent signals for body
    items.start_indent();

    // Separate enum constants, comments, and body declarations
    let enum_constants: Vec<_> = members
        .iter()
        .filter(|c| c.kind() == "enum_constant")
        .collect();
    let has_body_decls = members
        .iter()
        .any(|c| c.kind() == "enum_body_declarations" || c.kind() == ";");

    // Check if source has a trailing comma after the last enum constant.
    // Look for a "," child immediately before ";" or "enum_body_declarations".
    let has_trailing_comma = {
        let non_extra: Vec<_> = members.iter().filter(|c| !c.is_extra()).collect();
        non_extra.windows(2).any(|w| {
            w[0].kind() == ","
                && (w[1].kind() == ";" || w[1].kind() == "enum_body_declarations")
        })
    };

    let mut constant_idx = 0;
    let mut prev_was_constant = false;
    // Track previous member end row for source blank line detection
    let enum_open_brace_row = children
        .iter()
        .find(|c| c.kind() == "{")
        .map(|c| c.end_position().row);
    let mut enum_prev_end_row: Option<usize> = enum_open_brace_row;

    for child in &members {
        // Handle comments (extra nodes) without disrupting enum constant state
        if child.is_extra() {
            items.newline();
            // Preserve source blank lines before comments in enum body
            if enum_prev_end_row.is_some_and(|r| child.start_position().row > r + 1) {
                items.newline();
            }
            items.extend(gen_node(**child, context));
            enum_prev_end_row = Some(child.end_position().row);
            continue;
        }

        match child.kind() {
            "enum_constant" => {
                items.newline();
                // Preserve source blank lines before enum constants
                if enum_prev_end_row.is_some_and(|r| child.start_position().row > r + 1) {
                    items.newline();
                }
                items.extend(gen_enum_constant(**child, context));
                constant_idx += 1;
                let is_last = constant_idx == enum_constants.len();
                if !is_last {
                    items.push_str(",");
                } else if has_trailing_comma {
                    // Source had trailing comma after last constant — preserve it.
                    // PJF keeps trailing comma on last constant.
                    items.push_str(",");
                }
                prev_was_constant = true;
                enum_prev_end_row = Some(child.end_position().row);
            }
            "," => {
                // Tree-sitter may emit commas as anonymous tokens; skip
                // since we handle commas ourselves above.
            }
            ";" => {
                // PJF puts the semicolon on its own line after the last constant
                if prev_was_constant {
                    items.newline();
                }
                items.push_str(";");
                prev_was_constant = false;
            }
            "enum_body_declarations" => {
                // Tree-sitter wraps post-semicolon enum members in this node.
                // Use gen_body_with_members logic for source blank line preservation.
                let mut decl_cursor = child.walk();
                let decl_children: Vec<_> = child.children(&mut decl_cursor).collect();
                let mut decl_prev_end_row: Option<usize> = None;
                let mut decl_prev_was_line_comment = false;
                let mut decl_prev_was_block: Option<bool> = None;
                for decl_child in &decl_children {
                    if decl_child.kind() == ";" {
                        // PJF puts the semicolon on its own line when there's a trailing comma
                        if prev_was_constant && has_trailing_comma {
                            items.newline();
                        }
                        items.push_str(";");
                        decl_prev_end_row = Some(decl_child.end_position().row);
                        prev_was_constant = false;
                        continue;
                    }
                    if decl_child.is_extra() {
                        if !decl_prev_was_line_comment {
                            items.newline();
                        }
                        // Preserve source blank lines between comments
                        if let Some(prev_row) = decl_prev_end_row
                            && decl_child.start_position().row > prev_row + 1
                        {
                            items.newline();
                        }
                        items.extend(gen_node(*decl_child, context));
                        decl_prev_was_line_comment = decl_child.kind() == "line_comment";
                        decl_prev_end_row = Some(decl_child.end_position().row);
                        continue;
                    }
                    if decl_child.is_named() {
                        if !decl_prev_was_line_comment {
                            items.newline();
                        }
                        // Blank line from source or from block member adjacency
                        let source_blank = decl_prev_end_row
                            .is_some_and(|prev| decl_child.start_position().row > prev + 1);
                        let block_blank = match decl_prev_was_block {
                            None => false,
                            Some(prev_b) => prev_b || is_block_member(decl_child),
                        };
                        if source_blank || block_blank {
                            items.newline();
                        }
                        items.extend(gen_node(*decl_child, context));
                        decl_prev_was_line_comment = false;
                        decl_prev_was_block = Some(is_block_member(decl_child));
                        decl_prev_end_row = Some(decl_child.end_position().row);
                    }
                }
                prev_was_constant = false;
            }
            _ if child.is_named() => {
                if prev_was_constant {
                    items.push_str(";");
                    prev_was_constant = false;
                }
                items.newline();
                items.newline();
                items.extend(gen_node(**child, context));
            }
            _ => {}
        }
    }

    // If there were only constants and no explicit semicolon/body declarations,
    // add a trailing comma on the last constant (Java convention)
    let _ = has_body_decls;

    items.finish_indent();
    items.newline();
    items.push_str("}");

    items
}

/// Format a single enum constant.
fn gen_enum_constant<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                if !ends_with_newline {
                    items.space();
                }
            }
            "identifier" => {
                items.extend(gen_node_text(child, context.source));
            }
            "argument_list" => {
                items.extend(gen_node(child, context));
            }
            "class_body" => {
                items.space();
                items.extend(gen_class_body(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format formal parameters: `(Type name, Type name)`
///
/// If the parameter list would exceed `line_width`, wraps with 8-space
/// continuation indent (PJF style):
/// ```java
/// public void method(
///         String param1,
///         String param2) {
/// ```
#[allow(clippy::too_many_lines)]
pub fn gen_formal_parameters<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    let params: Vec<_> = children
        .iter()
        .filter(|c| {
            c.kind() == "formal_parameter"
                || c.kind() == "spread_parameter"
                || c.kind() == "receiver_parameter"
        })
        .collect();

    // Collect comment (extra) nodes between parameters, keyed by the byte offset
    // of the NEXT named param they precede.
    let mut comments_before_param: std::collections::HashMap<usize, Vec<tree_sitter::Node>> =
        std::collections::HashMap::new();
    {
        let mut pending_comments: Vec<tree_sitter::Node> = Vec::new();
        for child in &children {
            if child.is_extra() {
                pending_comments.push(*child);
            } else if (child.kind() == "formal_parameter"
                || child.kind() == "spread_parameter"
                || child.kind() == "receiver_parameter")
                && !pending_comments.is_empty()
            {
                comments_before_param.insert(child.start_byte(), pending_comments.clone());
                pending_comments.clear();
            }
        }
        if !pending_comments.is_empty() {
            comments_before_param.insert(usize::MAX, pending_comments);
        }
    }
    let has_interleaved_comments = !comments_before_param.is_empty();

    // Calculate total inline width of params (stable: uses indent_level, not source column)
    let param_text_width: usize = params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let text = &context.source[p.start_byte()..p.end_byte()];
            let flat: usize = text.lines().map(|l| l.trim().len()).sum();
            flat + if i < params.len() - 1 { 2 } else { 0 }
        })
        .sum();
    let indent_width = context.indent_level() * context.config.indent_width as usize;

    // Account for the prefix width (method name, return type, etc.) on the same line.
    // If the method name was wrapped to a continuation line, use the override prefix width.
    let prefix_width = context.take_override_prefix_width().unwrap_or_else(|| {
        estimate_prefix_width(node, context.source, context.is_assignment_wrapped())
    });

    // Suffix after closing paren: ") {" for methods/constructors with body (+4 for "(" + ") {"),
    // ");" for abstract methods (+3 for "(" + ");"), default +4 for safety.
    let suffix_width = match node.parent().map(|p| p.kind()) {
        Some("method_declaration" | "constructor_declaration") => {
            // Check if the method has a body (block) or throws clause following params.
            // If it has a throws clause, that adds more but wraps separately.
            // Just account for the `) {` or `);` suffix.
            let parent = node.parent().unwrap();
            let has_body = parent.child_by_field_name("body").is_some();
            if has_body { 4 } else { 3 } // "() {" vs "();"
        }
        _ => 2, // Just "()" for other contexts
    };

    let should_wrap = has_interleaved_comments
        || indent_width + prefix_width + param_text_width + suffix_width
            > context.config.line_width as usize;

    items.push_str("(");

    if should_wrap {
        // PJF bin-packing: first try putting ALL params on one continuation line.
        // If they fit, use single-line continuation. If not, fall back to one-per-line.
        let continuation_col = indent_width + 2 * (context.config.indent_width as usize);
        // Account for suffix after ): typically " {" for methods/constructors = 3 chars (") {")
        // PJF allows lines up to exactly line_width (120), so use <= not <
        let all_fit_continuation = !has_interleaved_comments
            && continuation_col + param_text_width + 3 <= context.config.line_width as usize;

        // 2x StartIndent for 8-space continuation indent
        items.start_indent();
        items.start_indent();

        if all_fit_continuation {
            // All params fit on one continuation-indent line (PJF bin-packing mode)
            items.newline();
            for (i, param) in params.iter().enumerate() {
                items.extend(gen_node(**param, context));
                if i < params.len() - 1 {
                    items.push_str(",");
                    items.space();
                }
            }
        } else {
            // One-per-line (too long even at continuation indent)
            let continuation_col = indent_width + 2 * (context.config.indent_width as usize);
            for (i, param) in params.iter().enumerate() {
                // Emit any comments that precede this parameter
                let has_preceding_comment =
                    comments_before_param.contains_key(&param.start_byte());
                if let Some(cmnts) = comments_before_param.get(&param.start_byte()) {
                    for cmnt in cmnts {
                        items.newline();
                        items.extend(gen_node(*cmnt, context));
                    }
                }
                // Only emit NewLine before param if no comment preceded it
                if !has_preceding_comment {
                    items.newline();
                }

                // Check if this param exceeds line_width at continuation indent.
                // If so, split after annotations: put type+name on next line at +8.
                let param_text = &context.source[param.start_byte()..param.end_byte()];
                let param_flat_width: usize =
                    param_text.lines().map(|l| l.trim().len()).sum();
                let suffix = usize::from(i < params.len() - 1); // comma
                if continuation_col + param_flat_width + suffix
                    > context.config.line_width as usize
                {
                    // Find the last annotation child — break after it
                    let mut pc = param.walk();
                    let param_children: Vec<_> = param.children(&mut pc).collect();
                    let has_modifiers =
                        param_children.iter().any(|c| c.kind() == "modifiers");
                    if has_modifiers {
                        // Emit modifiers (annotations), then wrap, then type+name
                        // on the same continuation line.
                        let mut past_modifiers = false;
                        let mut started_continuation = false;
                        for child in &param_children {
                            if child.kind() == "modifiers" {
                                items.extend(gen_node(*child, context));
                            } else {
                                if !started_continuation {
                                    items.start_indent();
                                    items.start_indent();
                                    items.newline();
                                    started_continuation = true;
                                    past_modifiers = true;
                                }
                                if past_modifiers && child.kind() != "modifiers" {
                                    // Space between type and name (but not before first)
                                    if child.kind() == "identifier"
                                        || child.kind() == "variable_declarator"
                                    {
                                        items.space();
                                    }
                                    items.extend(gen_node(*child, context));
                                }
                            }
                        }
                        if started_continuation {
                            items.finish_indent();
                            items.finish_indent();
                        }
                    } else {
                        items.extend(gen_node(**param, context));
                    }
                } else {
                    items.extend(gen_node(**param, context));
                }
                if i < params.len() - 1 {
                    items.push_str(",");
                }
            }
            // Trailing comments after last param
            if let Some(cmnts) = comments_before_param.get(&usize::MAX) {
                for cmnt in cmnts {
                    items.newline();
                    items.extend(gen_node(*cmnt, context));
                }
            }
        }
        items.push_str(")");
        items.finish_indent();
        items.finish_indent();
    } else {
        for (i, param) in params.iter().enumerate() {
            items.extend(gen_node(**param, context));
            if i < params.len() - 1 {
                items.push_str(",");
                items.space();
            }
        }
        items.push_str(")");
    }

    items
}

/// Format `throws Exception1, Exception2`
///
/// When the throws list would cause the line to exceed `line_width`, wraps at
/// commas with continuation indent (PJF style):
/// ```java
/// throws NoSuchFieldException, IllegalArgumentException,
///         UnsupportedOperationException, IOException {
/// ```
fn gen_throws<'a>(node: tree_sitter::Node<'a>, context: &mut FormattingContext<'a>) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    // Collect exception types
    let types: Vec<_> = node
        .children(&mut cursor)
        .filter(tree_sitter::Node::is_named)
        .collect();

    // Compute flat width of entire throws clause: "throws Type1, Type2, ..."
    let types_flat_width: usize = types
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let text = &context.source[t.start_byte()..t.end_byte()];
            text.len() + if i < types.len() - 1 { 2 } else { 0 } // ", "
        })
        .sum();

    // Use effective indent level to account for continuation indent when throws
    // is on a wrapped line. Add "throws " (7) prefix and " {" (2) suffix.
    let indent_width = context.effective_indent_level() * context.config.indent_width as usize;
    let line_width = context.config.line_width as usize;

    // Check if the full throws clause fits on the current line.
    // When throws is on a continuation line (after wrapped params), the effective
    // indent already includes the continuation indent.
    let needs_wrap = indent_width + 7 + types_flat_width + 2 > line_width;

    items.push_str("throws");

    if needs_wrap && types.len() > 1 {
        // Bin-pack exceptions: fill up the current line, then wrap remaining
        let continuation_col = indent_width + 2 * (context.config.indent_width as usize);
        let mut current_line_width = indent_width + 7; // "throws "
        for (i, typ) in types.iter().enumerate() {
            let text = &context.source[typ.start_byte()..typ.end_byte()];
            let type_width = text.len();

            if i > 0 && current_line_width + type_width + 2 > line_width {
                // +2 for suffix (" {" or ", "). Wrap to continuation line.
                items.start_indent();
                items.start_indent();
                items.newline();
                items.extend(gen_node(*typ, context));
                if i < types.len() - 1 {
                    items.push_str(",");
                }
                items.finish_indent();
                items.finish_indent();
                current_line_width = continuation_col + type_width + 2;
            } else {
                items.space();
                items.extend(gen_node(*typ, context));
                if i < types.len() - 1 {
                    items.push_str(",");
                }
                current_line_width += 1 + type_width + 2; // space + type + ", "
            }
        }
    } else {
        // Simple inline: "throws Type1, Type2"
        for (i, typ) in types.iter().enumerate() {
            if i == 0 {
                items.space();
            }
            items.extend(gen_node(*typ, context));
            if i < types.len() - 1 {
                items.push_str(",");
                items.space();
            }
        }
    }

    items
}

/// Format a variable declarator: `name = value`
///
/// When the full declaration (type + name + = + value) exceeds `line_width`,
/// wraps after `=` with 8-space continuation indent (PJF style):
/// ```java
/// VeryLongType<Generic> variable =
///         new VeryLongType<>(args);
/// ```
#[allow(clippy::too_many_lines)]
pub fn gen_variable_declarator<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Check if the full declaration line would exceed line_width.
    // Walk the parent node's children to reconstruct the flat width accurately,
    // mirroring how gen_field_declaration / gen_local_variable_declaration build the line.
    let has_value = children.iter().any(|c| c.kind() == "=");
    // If the value is an array_initializer with comments, skip variable declarator wrapping.
    // The array_initializer will expand to multiple lines on its own.
    let value_is_array_with_comments = children.iter().any(|c| {
        if c.kind() == "array_initializer" {
            let mut cursor = c.walk();
            c.children(&mut cursor).any(|ch| ch.is_extra())
        } else {
            false
        }
    });
    // PJF-style assignment wrapping: only break at `=` when the RHS expression
    // itself would be multi-line (i.e., wouldn't fit on one line even at continuation
    // indent). This matches PJF's `breakOnlyIfInnerLevelsThenFitOnOneLine` behavior.
    //
    // If the RHS is a single expression that fits on one line (even if the total line
    // with LHS exceeds line_width), we do NOT wrap at `=`.
    let wrap_value = has_value && !value_is_array_with_comments && {
        // Find the RHS value expression (the named child after `=`)
        let mut found_eq = false;
        let value_node = children.iter().find(|c| {
            if c.kind() == "=" {
                found_eq = true;
                return false;
            }
            found_eq && c.is_named()
        });

        if let Some(val) = value_node {
            // Compute the flat width of just the RHS expression (collapse whitespace
            // to get the "on one line" width)
            let val_text = &context.source[val.start_byte()..val.end_byte()];
            let rhs_flat_width = collapse_whitespace_len(val_text);

            let indent_unit = context.config.indent_width as usize;
            let indent_col = context.indent_level() * indent_unit;
            // Continuation indent: current indent + 2 indent units (double indent for wrapping)
            let continuation_indent = indent_col + indent_unit * 2;
            let line_width = context.config.line_width as usize;

            // Compute LHS width: type + variable name (everything before the `=` sign).
            // We need to look at the parent node to get the type information.
            let lhs_width = if context.is_declarator_on_new_line() {
                // The declarator starts on a continuation line; only count its own LHS.
                let mut w = 0;
                for c in &children {
                    if c.kind() == "=" {
                        break;
                    }
                    let text = &context.source[c.start_byte()..c.end_byte()];
                    if w > 0 {
                        w += 1;
                    }
                    w += collapse_whitespace_len(text);
                }
                w
            } else if let Some(parent) = node.parent() {
                let mut w = 0;
                let mut cursor = parent.walk();

                for c in parent.children(&mut cursor) {
                    // Skip until we find our variable_declarator
                    if c == node {
                        // Now add the variable_declarator's children up to the `=`
                        for vc in &children {
                            if vc.kind() == "=" {
                                break;
                            }
                            let text = &context.source[vc.start_byte()..vc.end_byte()];
                            if w > 0 {
                                w += 1;
                            } // space between tokens
                            w += collapse_whitespace_len(text);
                        }
                        break;
                    }

                    // Accumulate width from type, modifiers, etc. before variable_declarator
                    if c.is_named() {
                        let text = &context.source[c.start_byte()..c.end_byte()];
                        if w > 0 {
                            w += 1;
                        } // space between tokens
                        w += collapse_whitespace_len(text);
                    }
                }
                w
            } else {
                // Fallback: just the variable_declarator's LHS parts
                let mut w = 0;
                for c in &children {
                    if c.kind() == "=" {
                        break;
                    }
                    let text = &context.source[c.start_byte()..c.end_byte()];
                    if w > 0 {
                        w += 1;
                    }
                    w += collapse_whitespace_len(text);
                }
                w
            };

            // PJF-style chain assignment: prefer wrapping at '=' over wrapping the chain.
            // Use flatten_chain to get the TRUE chain root and first segment.
            let is_chain = val.kind() == "method_invocation" && expressions::chain_depth(*val) >= 1;

            if is_chain {
                let (root_width, first_seg_width) =
                    expressions::chain_root_first_seg_width(*val, context.source);

                // Check if `LHS = root.firstMethod()` fits on one line
                let lhs_plus_first_seg = indent_col + lhs_width + 3 + root_width + first_seg_width;

                if lhs_plus_first_seg > line_width {
                    // First segment doesn't fit -> must wrap at =
                    true
                } else {
                    // PJF preference: if chain WOULD wrap at current position,
                    // check if wrapping at '=' allows the chain to stay inline.
                    let current_col = indent_col + lhs_width + 3; // after "LHS = "
                    let chain_fits_current = expressions::chain_fits_inline_at(
                        *val,
                        current_col,
                        context.source,
                        context.config,
                    );
                    if chain_fits_current {
                        false // Chain fits at current position, no wrapping needed
                    } else {
                        // Chain would wrap at current position. Check if it fits
                        // inline at continuation indent — if so, wrap at '='.
                        let continuation_col =
                            indent_col + 2 * (context.config.indent_width as usize);
                        expressions::chain_fits_inline_at(
                            *val,
                            continuation_col,
                            context.source,
                            context.config,
                        )
                    }
                }
            } else {
                // Anonymous class bodies always wrap at `=` (they're inherently multi-line)
                let is_anonymous_class = val.kind() == "object_creation_expression" && {
                    let mut vc = val.walk();
                    val.children(&mut vc).any(|c| c.kind() == "class_body")
                };
                if is_anonymous_class {
                    let total_line_width = indent_col + lhs_width + 3 + rhs_flat_width + 1;
                    total_line_width > line_width
                } else {
                    // Ternary and binary expressions usually wrap at their own operators
                    // (`?`/`:` or `&&`/`||`). But for ternaries that fit on a continuation
                    // line, prefer wrapping at `=` (PJF style).
                    let is_ternary = matches!(val.kind(), "ternary_expression" | "conditional_expression");
                    let is_binary = val.kind() == "binary_expression";
                    if is_ternary {
                        let total_line_width = indent_col + lhs_width + 3 + rhs_flat_width + 1;
                        let rhs_fits_at_continuation =
                            continuation_indent + rhs_flat_width <= line_width;
                        total_line_width > line_width && rhs_fits_at_continuation
                    } else if is_binary {
                        false
                    } else {
                        // PJF-style: only break at `=` when the RHS fits on one continuation
                        // line. If the RHS itself is too wide, keep `= expr(` inline and let
                        // the expression's internal wrapping (arg list, etc.) handle it.
                        let rhs_fits_at_continuation =
                            continuation_indent + rhs_flat_width <= line_width;
                        let total_line_width = indent_col + lhs_width + 3 + rhs_flat_width + 1;
                        let total_too_wide = total_line_width > line_width;
                        if rhs_fits_at_continuation && total_too_wide {
                            true
                        } else if !rhs_fits_at_continuation && total_too_wide {
                            // RHS is too wide for continuation, but check if keeping
                            // `LHS = opening(` inline also exceeds line_width.
                            // If so, we must wrap at `=` to avoid >line_width lines.
                            let rhs_text = &context.source[val.start_byte()..val.end_byte()];
                            let rhs_opening_width =
                                rhs_text.find('(').map_or(rhs_flat_width, |p| p + 1);
                            let opening_line_width =
                                indent_col + lhs_width + 3 + rhs_opening_width;
                            opening_line_width > line_width
                        } else {
                            false
                        }
                    }
                }
            }
        } else {
            false
        }
    };

    let mut saw_eq = false;
    let mut cursor2 = node.walk();
    for child in node.children(&mut cursor2) {
        match child.kind() {
            "identifier" | "dimensions" => {
                items.extend(gen_node_text(child, context.source));
            }
            "=" => {
                items.space();
                items.push_str("=");
                saw_eq = true;
                if wrap_value {
                    items.start_indent();
                    items.start_indent();
                    items.newline();
                } else {
                    items.space();
                }
            }
            _ if child.is_named() => {
                // If we wrapped at '=', tell downstream that the assignment is
                // on a different line (prefix width should not include LHS)
                if wrap_value && saw_eq {
                    context.set_assignment_wrapped(true);
                }
                items.extend(gen_node(child, context));
                if wrap_value && saw_eq {
                    context.set_assignment_wrapped(false);
                }
            }
            _ => {}
        }
    }

    if wrap_value && saw_eq {
        items.finish_indent();
        items.finish_indent();
    }

    items
}

/// Format an argument list: `(arg1, arg2, arg3)`
///
/// Wraps with 8-space continuation indent when the argument list would
/// exceed `line_width`. Uses stable width estimation based on `context.indent_level()`
/// to avoid instability between formatting passes.
///
/// When wrapping, uses PJF-style "bin-packing": tries to fit all args on one
/// continuation line first, only putting each arg on its own line if they don't fit.
#[allow(clippy::too_many_lines)]
pub fn gen_argument_list<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    let args: Vec<_> = children
        .iter()
        .filter(|c| c.is_named() && !c.is_extra())
        .collect();

    // Collect comment (extra) nodes between arguments, keyed by the byte offset
    // of the NEXT named arg they precede. Comments before the first arg are keyed
    // by the first arg's start_byte.
    let mut comments_before_arg: std::collections::HashMap<usize, Vec<tree_sitter::Node>> =
        std::collections::HashMap::new();
    {
        let mut pending_comments: Vec<tree_sitter::Node> = Vec::new();
        for child in &children {
            if child.is_extra() {
                pending_comments.push(*child);
            } else if child.is_named() && !pending_comments.is_empty() {
                comments_before_arg.insert(child.start_byte(), pending_comments.clone());
                pending_comments.clear();
            }
        }
        // Comments after the last arg (before ')') — attach to a sentinel key
        if !pending_comments.is_empty() {
            comments_before_arg.insert(usize::MAX, pending_comments);
        }
    }
    let has_interleaved_comments = !comments_before_arg.is_empty();

    // Estimate the "flat" width of arguments (stripping embedded newlines).
    // For lambda expressions with block bodies, only count the header (params -> {)
    // since the block body will always be on separate lines.
    let args_flat_width: usize = args
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let width = if a.kind() == "lambda_expression" {
                // Find the block body child — if present, only measure up to "{"
                let mut cursor = a.walk();
                let has_block = a.children(&mut cursor).any(|c| c.kind() == "block");
                if has_block {
                    // Lambda header: params + " -> {"
                    let mut cursor2 = a.walk();
                    let mut header_width = 0;
                    for child in a.children(&mut cursor2) {
                        if child.kind() == "block" {
                            header_width += 1; // the "{"
                            break;
                        }
                        let text = &context.source[child.start_byte()..child.end_byte()];
                        if child.kind() == "->" {
                            header_width += 4; // " -> "
                        } else {
                            header_width += text.len();
                        }
                    }
                    header_width
                } else {
                    let text = &context.source[a.start_byte()..a.end_byte()];
                    text.lines().map(|l| l.trim().len()).sum()
                }
            } else {
                let text = &context.source[a.start_byte()..a.end_byte()];
                text.lines().map(|l| l.trim().len()).sum()
            };
            width + if i < args.len() - 1 { 2 } else { 0 }
        })
        .sum();

    // Detect if this argument_list is inside a chained method call.
    // A call is "in a chain" if its parent method_invocation has a chained receiver
    // (receiver is itself a method_invocation) or is itself a receiver in a chain
    // (parent MI's parent is also a MI).
    let is_in_chain = node.parent().is_some_and(|p| {
        p.kind() == "method_invocation"
            && (p
                .child_by_field_name("object")
                .is_some_and(|obj| obj.kind() == "method_invocation")
                || p.parent()
                    .is_some_and(|gp| gp.kind() == "method_invocation"))
    });

    // Use effective indent level (including continuation indent from wrapped chains
    // and wrapped argument lists) to get the true column position.
    let indent_level = context.effective_indent_level();
    let indent_width = indent_level * context.config.indent_width as usize;
    let prefix_width = if is_in_chain {
        // Inside a chain, the chain wrapper handles overall layout.
        // Use only the immediate method/constructor name as prefix, not the full chain text.
        let parent_node = node.parent();
        let name_width = parent_node
            .and_then(|p| p.child_by_field_name("name"))
            .map_or(0, |n| {
                let text = &context.source[n.start_byte()..n.end_byte()];
                text.len()
            });
        let type_args_width = parent_node
            .and_then(|p| p.child_by_field_name("type_arguments"))
            .map_or(0, |ta| {
                let text = &context.source[ta.start_byte()..ta.end_byte()];
                collapse_whitespace_len(text)
            });
        1 + type_args_width + name_width // "." + type_args + name
    } else {
        // Check if the caller (e.g., an outer gen_argument_list) set an override
        // to communicate the true column position for nested calls.
        context.take_override_prefix_width().unwrap_or_else(|| {
            estimate_prefix_width(node, context.source, context.is_assignment_wrapped())
        })
    };

    // For single-arg calls where the arg is itself a call expression,
    // compute the "head width" (up to the inner call's opening paren).
    // PJF keeps `outer(inner(` on one line and lets the inner call wrap.
    let single_arg_head_width: Option<usize> = if args.len() == 1
        && matches!(
            args[0].kind(),
            "object_creation_expression" | "method_invocation"
        ) {
        args[0].child_by_field_name("arguments").map(|arg_args| {
            let head_text = &context.source[args[0].start_byte()..arg_args.start_byte()];
            collapse_whitespace_len(head_text) + 1 // +1 for "("
        })
    } else {
        None
    };

    // Check if args fit on the same line as the prefix.
    let mut fits_on_one_line = if args.is_empty() {
        true
    } else if args.len() == 1 && is_in_chain {
        // For single-arg calls in chains, keep inline — the chain handles layout.
        true
    } else if let Some(head_width) = single_arg_head_width {
        // Single-arg method/constructor: PJF's approach —
        // 1. If the full arg fits on a continuation line, wrap at outer level (normal)
        // 2. If it doesn't fit, keep outer(inner( inline and let inner wrap
        let continuation_indent = indent_width + (2 * context.config.indent_width as usize);
        let arg_fits_on_continuation =
            continuation_indent + args_flat_width + 1 < context.config.line_width as usize;
        if arg_fits_on_continuation {
            // Arg fits on continuation — use normal wrapping logic
            indent_width + prefix_width + args_flat_width + 2 < context.config.line_width as usize
        } else {
            // Arg doesn't fit on continuation — keep outer(inner( inline
            indent_width + prefix_width + head_width < context.config.line_width as usize
        }
    } else if args.len() == 1 && args[0].kind() == "binary_expression" {
        // Single-arg binary expressions (string concat, arithmetic, etc.) always
        // stay inline after '('. The binary expression wraps at its operators.
        true
    } else {
        indent_width + prefix_width + args_flat_width + 2 < context.config.line_width as usize
    };

    // Comments between arguments force one-per-line wrapping
    if has_interleaved_comments {
        fits_on_one_line = false;
    }

    // PJF's preferBreakingLastInnerLevel: if any arg contains a method chain whose
    // last dot would exceed METHOD_CHAIN_COLUMN_LIMIT (80), force wrapping.
    // Check at both inline and continuation positions.
    let chain_threshold = context.config.method_chain_threshold as usize;

    // Helper: check if any arg's chain dot exceeds threshold at given base column
    let exceeds_chain_limit = |base_col: usize| -> bool {
        let mut col = base_col;
        for arg in &args {
            let text = &context.source[arg.start_byte()..arg.end_byte()];
            let arg_width: usize = text.lines().map(|l| l.trim().len()).sum();
            let dot_pos = super::expressions::rightmost_chain_dot(**arg, context.source, col);
            if dot_pos > chain_threshold {
                return true;
            }
            col += arg_width + 2; // ", "
        }
        false
    };

    // Check at inline position: if chain dots exceed 80, break after "("
    // Skip for single-arg long chains (depth >= 3) — they will wrap at their
    // own dots, so forcing arg-list wrapping is unnecessary. Short chains
    // (depth 1-2) might stay inline, so the chain limit check still applies.
    let single_arg_is_long_chain = args.len() == 1
        && args[0].kind() == "method_invocation"
        && super::expressions::chain_depth(*args[0]) >= 3;
    if fits_on_one_line
        && !is_in_chain
        && !single_arg_is_long_chain
        && exceeds_chain_limit(indent_width + prefix_width)
    {
        fits_on_one_line = false;
    }

    // If not, check if args fit on ONE continuation line (8-space indent = 2 levels of indent_width)
    let continuation_indent = indent_width + (2 * context.config.indent_width as usize);
    let mut fits_on_continuation_line =
        continuation_indent + args_flat_width + 1 < context.config.line_width as usize;

    // Comments between arguments force one-per-line (can't bin-pack with comments)
    if has_interleaved_comments {
        fits_on_continuation_line = false;
    }

    // Also check at continuation position: if chain dots still exceed 80, force one-per-line
    if !fits_on_one_line
        && fits_on_continuation_line
        && args.len() > 1
        && exceeds_chain_limit(continuation_indent)
    {
        fits_on_continuation_line = false;
    }

    items.push_str("(");

    if fits_on_one_line {
        // Keep all args on the same line as the opening paren.
        // For single-arg call expressions where the arg doesn't fit on
        // continuation (inline-first-arg mode), set override so the inner
        // call knows its true column position for wrapping decisions.
        // Don't set override in chain context — chains handle their own layout.
        if !is_in_chain && let Some(head_width) = single_arg_head_width {
            let continuation_indent = indent_width + (2 * context.config.indent_width as usize);
            let arg_fits_on_continuation =
                continuation_indent + args_flat_width + 1 < context.config.line_width as usize;
            if !arg_fits_on_continuation {
                context.set_override_prefix_width(Some(prefix_width + head_width));
            }
        }
        for (i, arg) in args.iter().enumerate() {
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_str(",");
                items.space();
            }
        }
        // Clear any unconsumed override (e.g., when arg is a chain and
        // the override wasn't consumed by the chain's in-chain arg lists).
        context.set_override_prefix_width(None);
        items.push_str(")");
    } else if fits_on_continuation_line {
        // Wrap after opening paren, but put all args on ONE continuation line (bin-packing)
        items.start_indent();
        items.start_indent();
        items.newline();
        context.add_continuation_indent(2);
        for (i, arg) in args.iter().enumerate() {
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_str(",");
                items.space();
            }
        }
        context.remove_continuation_indent(2);
        items.push_str(")");
        items.finish_indent();
        items.finish_indent();
    } else {
        // Args don't fit on one continuation line, put each arg on its own line
        items.start_indent();
        items.start_indent();
        context.add_continuation_indent(2);
        for (i, arg) in args.iter().enumerate() {
            // Emit any comments that precede this arg
            if let Some(comments) = comments_before_arg.get(&arg.start_byte()) {
                for comment in comments {
                    items.newline();
                    items.extend(gen_node(*comment, context));
                }
            }
            items.newline();
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_str(",");
            }
        }
        // Emit any trailing comments (after last arg, before ')')
        if let Some(comments) = comments_before_arg.get(&usize::MAX) {
            for comment in comments {
                items.newline();
                items.extend(gen_node(*comment, context));
            }
        }
        context.remove_continuation_indent(2);
        items.push_str(")");
        items.finish_indent();
        items.finish_indent();
    }

    items
}

/// Generic handler for bodies with member declarations (`class_body`, `interface_body`, etc.)
///
/// Uses dprint-core's StartIndent/FinishIndent signals so that `NewLine`
/// automatically gets the correct indentation. Handles comment (extra) nodes
/// that appear between members.
/// Check if a class body member has a block body (ends with `}`).
/// Used to determine blank line insertion between members.
fn is_block_member(node: &tree_sitter::Node) -> bool {
    let kind = node.kind();
    if matches!(
        kind,
        "constructor_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "annotation_type_declaration"
            | "static_initializer"
            | "record_declaration"
            | "compact_constructor_declaration"
    ) {
        return true;
    }
    // All method declarations get blank lines between them (PJF behavior).
    // This includes abstract/interface methods without bodies.
    if kind == "method_declaration" {
        return true;
    }
    false
}

fn gen_body_with_members<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("{");

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Include both named members and extra (comment) nodes, excluding braces
    let members: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "{" && c.kind() != "}" && (c.is_named() || c.is_extra()))
        .collect();

    if members.is_empty() {
        items.push_str("}");
        return items;
    }

    items.start_indent();
    context.indent();

    let mut prev_was_line_comment = false;
    // Track whether previous member was a block member (has body ending with })
    let mut prev_was_block: Option<bool> = None; // None = first member after {
    // Track whether there was a comment between the previous member and current
    let mut had_comment_since_last_member = false;
    // Initialize to opening `{` row so we can detect source blank lines before first member
    let open_brace_row = children
        .iter()
        .find(|c| c.kind() == "{")
        .map(|c| c.end_position().row);
    let mut prev_end_row: Option<usize> = open_brace_row;

    for member in members.iter() {
        if member.is_extra() {
            let is_trailing = comments::is_trailing_comment(**member);
            if is_trailing {
                // Trailing comment: append on same line
                items.space();
                items.extend(gen_node(**member, context));
                prev_was_line_comment = member.kind() == "line_comment";
            } else {
                // Leading/standalone comment within body
                if !prev_was_line_comment {
                    items.newline();
                }
                // Add blank line before comment only if source has one.
                // PJF does NOT automatically add blanks before comments (javadoc etc.)
                // between block members — that blank is added before the actual member, not
                // before its leading comment.
                let source_has_blank =
                    prev_end_row.is_some_and(|prev_row| member.start_position().row > prev_row + 1);
                if source_has_blank {
                    items.newline();
                }
                items.extend(gen_node(**member, context));
                prev_was_line_comment = member.kind() == "line_comment";
                prev_end_row = Some(member.end_position().row);
                had_comment_since_last_member = true;
            }
            continue;
        }

        if !prev_was_line_comment {
            items.newline();
        }
        // Add blank line between class body members:
        // - Always from source blank lines
        // - Between block members (prev or cur has body ending with }), but ONLY if no
        //   comment intervened — PJF treats javadoc+method as one unit and doesn't add
        //   blank between end of javadoc and the method's annotation/modifiers.
        let source_has_blank =
            prev_end_row.is_some_and(|prev_row| member.start_position().row > prev_row + 1);
        let block_blank = if had_comment_since_last_member {
            false // comment between members: no automatic blank
        } else {
            match prev_was_block {
                None => false,
                Some(prev_block) => {
                    let cur_is_block = is_block_member(member);
                    prev_block || cur_is_block
                }
            }
        };
        if source_has_blank || block_blank {
            items.newline();
        }
        items.extend(gen_node(**member, context));

        prev_was_line_comment = false;
        prev_was_block = Some(is_block_member(member));
        prev_end_row = Some(member.end_position().row);
        had_comment_since_last_member = false;
    }

    items.finish_indent();
    context.dedent();
    if !prev_was_line_comment {
        items.newline();
    }
    // PJF removes source blank lines before closing `}` in class bodies.
    // (Statement blocks preserve them — handled separately in statements.rs.)
    items.push_str("}");

    items
}
