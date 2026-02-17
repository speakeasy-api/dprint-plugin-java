use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use super::comments;
use super::context::FormattingContext;
use super::expressions;
use super::generate::gen_node;
use super::helpers;

/// Format a package declaration: `package com.example;`
pub fn gen_package_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "package" => items.push_string("package".to_string()),
            "scoped_identifier" | "identifier" => {
                items.extend(helpers::gen_space());
                items.extend(helpers::gen_node_text(child, context.source));
            }
            ";" => items.push_string(";".to_string()),
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
            "import" => items.push_string("import".to_string()),
            "static" => {
                items.extend(helpers::gen_space());
                items.push_string("static".to_string());
            }
            "scoped_identifier" | "identifier" => {
                items.extend(helpers::gen_space());
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "asterisk" => {
                items.push_string(".*".to_string());
            }
            ";" => items.push_string(";".to_string()),
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
                    items.extend(helpers::gen_space());
                }
                items.push_string("class".to_string());
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = true;
            }
            "type_parameters" => {
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            "superclass" => {
                if wrap_extends {
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                    context.add_continuation_indent(2);
                    items.extend(gen_superclass(child, context));
                    context.remove_continuation_indent(2);
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                } else {
                    items.extend(helpers::gen_space());
                    items.extend(gen_superclass(child, context));
                }
                need_space = true;
            }
            "super_interfaces" => {
                if wrap_implements {
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                    context.add_continuation_indent(2);
                    items.extend(gen_super_interfaces(child, context));
                    context.remove_continuation_indent(2);
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                } else {
                    items.extend(helpers::gen_space());
                    items.extend(gen_super_interfaces(child, context));
                }
                need_space = true;
            }
            "class_body" => {
                items.extend(helpers::gen_space());
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
                    items.extend(helpers::gen_space());
                }
                items.push_string("interface".to_string());
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = true;
            }
            "type_parameters" => {
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            "extends_interfaces" => {
                if wrap_clauses {
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                    context.add_continuation_indent(2);
                    items.extend(gen_extends_interfaces(child, context));
                    context.remove_continuation_indent(2);
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                } else {
                    items.extend(helpers::gen_space());
                    items.extend(gen_extends_interfaces(child, context));
                }
                need_space = true;
            }
            "interface_body" => {
                items.extend(helpers::gen_space());
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
                    items.extend(helpers::gen_space());
                }
                items.push_string("enum".to_string());
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = true;
            }
            "super_interfaces" => {
                if wrap_clauses {
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                    items.extend(gen_super_interfaces(child, context));
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                } else {
                    items.extend(helpers::gen_space());
                    items.extend(gen_super_interfaces(child, context));
                }
                need_space = true;
            }
            "enum_body" => {
                items.extend(helpers::gen_space());
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
                    items.extend(helpers::gen_space());
                }
                items.push_string("record".to_string());
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = false;
            }
            "formal_parameters" => {
                items.extend(gen_formal_parameters(child, context));
                need_space = true;
            }
            "super_interfaces" => {
                if wrap_clauses {
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                    items.extend(gen_super_interfaces(child, context));
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                } else {
                    items.extend(helpers::gen_space());
                    items.extend(gen_super_interfaces(child, context));
                }
                need_space = true;
            }
            "class_body" => {
                items.extend(helpers::gen_space());
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
                    items.extend(helpers::gen_space());
                }
                items.push_string("@interface".to_string());
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = true;
            }
            "annotation_type_body" => {
                items.extend(helpers::gen_space());
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
pub fn gen_method_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    // Pre-calculate: estimate method signature line width to decide throws wrapping.
    // Compute width of everything up to and including `)` + throws clause.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let sig_width = estimate_method_sig_width(node, context.source);
    let line_width = context.config.line_width as usize;
    // +2 for the trailing " {" that follows the throws clause
    let wrap_throws = indent_width + sig_width + 2 > line_width;

    // PJF: wrap between return type and method name when the signature is too long.
    // Example: `public CompletableFuture<VeryLongResponse>\n        methodName(params) {`
    let wrap_before_name = {
        let mut cursor_pre = node.walk();
        let children_pre: Vec<_> = node.children(&mut cursor_pre).collect();
        // Find the method name (identifier) position
        let name_idx = children_pre
            .iter()
            .position(|c| c.kind() == "identifier");
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
            let name_text = &context.source[children_pre[idx].start_byte()
                ..children_pre[idx].end_byte()];
            let name_width = name_text.len();
            // Estimate params width
            let params_width: usize = children_pre.iter().find_map(|c| {
                if c.kind() == "formal_parameters" {
                    let text = &context.source[c.start_byte()..c.end_byte()];
                    Some(expressions::collapse_whitespace(text).len())
                } else {
                    None
                }
            }).unwrap_or(2); // "()" minimum
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
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            // Return type: various type nodes
            "void_type"
            | "integral_type"
            | "floating_point_type"
            | "boolean_type"
            | "type_identifier"
            | "scoped_type_identifier"
            | "generic_type"
            | "array_type" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_node(child, context));
                need_space = true;
            }
            "identifier" => {
                if wrap_before_name {
                    // Wrap: put method name on continuation-indent line
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                    did_wrap_name = true;
                    // Tell formal_parameters the effective prefix is just the method name
                    let name_text =
                        &context.source[child.start_byte()..child.end_byte()];
                    context.set_override_prefix_width(Some(name_text.len()));
                } else if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = false;
            }
            "formal_parameters" => {
                items.extend(gen_formal_parameters(child, context));
                need_space = true;
            }
            "throws" => {
                if wrap_throws {
                    if !did_wrap_name {
                        items.push_signal(Signal::StartIndent);
                        items.push_signal(Signal::StartIndent);
                    }
                    items.push_signal(Signal::NewLine);
                    items.extend(gen_throws(child, context));
                    if !did_wrap_name {
                        items.push_signal(Signal::FinishIndent);
                        items.push_signal(Signal::FinishIndent);
                    }
                } else {
                    items.extend(helpers::gen_space());
                    items.extend(gen_throws(child, context));
                }
                need_space = true;
            }
            "block" => {
                if did_wrap_name {
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                }
                items.extend(helpers::gen_space());
                items.extend(gen_node(child, context));
                need_space = false;
                did_wrap_name = false; // consumed
            }
            ";" => {
                if did_wrap_name {
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                    did_wrap_name = false;
                }
                items.push_string(";".to_string());
                need_space = false;
            }
            "dimensions" => {
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = true;
            }
            _ => {}
        }
    }

    if did_wrap_name {
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
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

/// Estimate the prefix width before a formal_parameters or argument_list node.
/// This is the text that appears on the same line before the opening `(`:
/// - For methods: modifiers + return type + method name
/// - For constructors: modifiers + constructor name
/// - For method invocations: receiver + method name
/// - For object creation: `new` + type name
///
/// Uses the parent-to-node text as the base measurement, then walks up
/// ancestors to account for keywords/LHS that share the same line.
fn estimate_prefix_width(node: tree_sitter::Node, source: &str) -> usize {
    let parent = match node.parent() {
        Some(p) => p,
        None => return 0,
    };

    // Extract the text from the start of the parent to the start of this node
    let prefix_text = &source[parent.start_byte()..node.start_byte()];

    // Only consider the last line to handle multiline modifiers/annotations
    let last_line = prefix_text.lines().last().unwrap_or(prefix_text);
    let mut width = last_line.trim_start().len();

    // Walk up ancestors to accumulate prefix from keywords/LHS that share the line.
    // Stop when we hit a node that may introduce a line break (e.g., variable_declarator
    // wraps at `=`, method_declaration can wrap return type from name).
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
                // Add LHS width: e.g., "this.baseUrl = " before the RHS
                let lhs_text = &source[anc.start_byte()..parent.start_byte()];
                let lhs_last_line = lhs_text.lines().last().unwrap_or(lhs_text);
                width += lhs_last_line.trim_start().len();
                break;
            }
            // These are wrapping boundaries — stop walking
            "variable_declarator" | "local_variable_declaration" | "field_declaration"
            | "method_declaration" | "constructor_declaration" => break,
            _ => {
                ancestor = anc.parent();
            }
        }
    }

    width
}

/// Estimate the width of a class/interface/enum/record declaration line
/// (modifiers + keyword + name + type_parameters + extends/implements + body start)
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
                let flat = expressions::collapse_whitespace(text);
                if width > 0
                    && child.kind() != "formal_parameters"
                    && child.kind() != "("
                    && child.kind() != ")"
                {
                    width += 1; // space separator
                }
                width += flat.len();
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
    // +2 for the trailing " {" that follows the throws clause
    let wrap_throws = indent_width + sig_width + 2 > context.config.line_width as usize;

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
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
                need_space = false;
            }
            "formal_parameters" => {
                items.extend(gen_formal_parameters(child, context));
                need_space = true;
            }
            "throws" => {
                if wrap_throws {
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                    items.extend(gen_throws(child, context));
                    items.push_signal(Signal::FinishIndent);
                    items.push_signal(Signal::FinishIndent);
                } else {
                    items.extend(helpers::gen_space());
                    items.extend(gen_throws(child, context));
                }
                need_space = true;
            }
            "constructor_body" => {
                items.extend(helpers::gen_space());
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

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                let (modifier_items, ends_with_newline) = gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            // Type nodes
            "void_type"
            | "integral_type"
            | "floating_point_type"
            | "boolean_type"
            | "type_identifier"
            | "scoped_type_identifier"
            | "generic_type"
            | "array_type" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_node(child, context));
                need_space = true;
            }
            "variable_declarator" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_variable_declarator(child, context));
                need_space = false;
            }
            "," => {
                items.push_string(",".to_string());
                need_space = true;
            }
            ";" => {
                items.push_string(";".to_string());
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
/// Returns (items, ends_with_newline) where ends_with_newline is true
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
        items.push_signal(Signal::NewLine);
    }

    // Emit keyword modifiers on a single line
    let mut first = true;
    for kw in &keywords {
        if !first {
            items.extend(helpers::gen_space());
        }
        items.extend(helpers::gen_node_text(**kw, context.source));
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
            "<" => items.push_string("<".to_string()),
            ">" => items.push_string(">".to_string()),
            "," => {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
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
            "extends" => items.push_string("extends".to_string()),
            _ if child.is_named() => {
                items.extend(helpers::gen_space());
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
                items.push_string("implements".to_string());
            }
            "type_list" => {
                items.extend(helpers::gen_space());
                items.extend(gen_type_list(child, context));
            }
            "," => {
                items.push_string(",".to_string());
            }
            _ if child.is_named() => {
                items.extend(helpers::gen_space());
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
                items.push_string("extends".to_string());
            }
            "type_list" => {
                items.extend(helpers::gen_space());
                items.extend(gen_type_list(child, context));
            }
            "," => {
                items.push_string(",".to_string());
            }
            _ if child.is_named() => {
                items.extend(helpers::gen_space());
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
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
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
fn gen_enum_body<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_string("{".to_string());

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Collect children excluding braces
    let members: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "{" && c.kind() != "}")
        .collect();

    if members.is_empty() {
        items.push_string("}".to_string());
        return items;
    }

    // Use dprint-core indent signals for body
    items.push_signal(Signal::StartIndent);

    // Separate enum constants, comments, and body declarations
    let enum_constants: Vec<_> = members
        .iter()
        .filter(|c| c.kind() == "enum_constant")
        .collect();
    let has_body_decls = members
        .iter()
        .any(|c| c.kind() == "enum_body_declarations" || c.kind() == ";");

    let mut constant_idx = 0;
    let mut prev_was_constant = false;

    for child in &members {
        // Handle comments (extra nodes) without disrupting enum constant state
        if child.is_extra() {
            items.push_signal(Signal::NewLine);
            items.extend(gen_node(**child, context));
            continue;
        }

        match child.kind() {
            "enum_constant" => {
                items.push_signal(Signal::NewLine);
                items.extend(gen_enum_constant(**child, context));
                constant_idx += 1;
                let is_last = constant_idx == enum_constants.len();
                if !is_last {
                    items.push_string(",".to_string());
                }
                prev_was_constant = true;
            }
            "," => {
                // Tree-sitter may emit commas as anonymous tokens; skip
                // since we handle commas ourselves above.
            }
            ";" => {
                items.push_string(";".to_string());
                prev_was_constant = false;
            }
            "enum_body_declarations" => {
                // Tree-sitter wraps post-semicolon enum members in this node.
                let mut decl_cursor = child.walk();
                for decl_child in child.children(&mut decl_cursor) {
                    match decl_child.kind() {
                        ";" => {
                            items.push_string(";".to_string());
                        }
                        _ if decl_child.is_named() => {
                            items.push_signal(Signal::NewLine);
                            items.push_signal(Signal::NewLine);
                            items.extend(gen_node(decl_child, context));
                        }
                        _ => {}
                    }
                }
                prev_was_constant = false;
            }
            _ if child.is_named() => {
                if prev_was_constant {
                    items.push_string(";".to_string());
                    prev_was_constant = false;
                }
                items.push_signal(Signal::NewLine);
                items.push_signal(Signal::NewLine);
                items.extend(gen_node(**child, context));
            }
            _ => {}
        }
    }

    // If there were only constants and no explicit semicolon/body declarations,
    // add a trailing comma on the last constant (Java convention)
    let _ = has_body_decls;

    items.push_signal(Signal::FinishIndent);
    items.push_signal(Signal::NewLine);
    items.push_string("}".to_string());

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
                    items.extend(helpers::gen_space());
                }
            }
            "identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "argument_list" => {
                items.extend(gen_node(child, context));
            }
            "class_body" => {
                items.extend(helpers::gen_space());
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
    let prefix_width = context
        .take_override_prefix_width()
        .unwrap_or_else(|| estimate_prefix_width(node, context.source));

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

    let should_wrap =
        indent_width + prefix_width + param_text_width + suffix_width
            >= context.config.line_width as usize;

    items.push_string("(".to_string());

    if should_wrap {
        // PJF bin-packing: first try putting ALL params on one continuation line.
        // If they fit, use single-line continuation. If not, fall back to one-per-line.
        let continuation_col = indent_width + 2 * (context.config.indent_width as usize);
        // Account for suffix after ): typically " {" for methods/constructors = 3 chars (") {")
        // Use +3 instead of +1 and strict < to match PJF behavior
        let all_fit_continuation =
            continuation_col + param_text_width + 3 < context.config.line_width as usize;

        // 2x StartIndent for 8-space continuation indent
        items.push_signal(Signal::StartIndent);
        items.push_signal(Signal::StartIndent);

        if all_fit_continuation {
            // All params fit on one continuation-indent line (PJF bin-packing mode)
            items.push_signal(Signal::NewLine);
            for (i, param) in params.iter().enumerate() {
                items.extend(gen_node(**param, context));
                if i < params.len() - 1 {
                    items.push_string(",".to_string());
                    items.extend(helpers::gen_space());
                }
            }
        } else {
            // One-per-line (too long even at continuation indent)
            for (i, param) in params.iter().enumerate() {
                items.push_signal(Signal::NewLine);
                items.extend(gen_node(**param, context));
                if i < params.len() - 1 {
                    items.push_string(",".to_string());
                }
            }
        }
        items.push_string(")".to_string());
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
    } else {
        for (i, param) in params.iter().enumerate() {
            items.extend(gen_node(**param, context));
            if i < params.len() - 1 {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
            }
        }
        items.push_string(")".to_string());
    }

    items
}

/// Format `throws Exception1, Exception2`
fn gen_throws<'a>(node: tree_sitter::Node<'a>, context: &mut FormattingContext<'a>) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut first_type = true;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "throws" => items.push_string("throws".to_string()),
            "," => {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
            }
            _ if child.is_named() => {
                // Only add space before first type (after "throws")
                if first_type {
                    items.extend(helpers::gen_space());
                    first_type = false;
                }
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a variable declarator: `name = value`
///
/// When the full declaration (type + name + = + value) exceeds line_width,
/// wraps after `=` with 8-space continuation indent (PJF style):
/// ```java
/// VeryLongType<Generic> variable =
///         new VeryLongType<>(args);
/// ```
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
    // If the value is a ternary expression, skip variable declarator wrapping.
    // The ternary's own wrapping logic will handle line-breaking before ? and :.
    let value_is_ternary = children.iter().any(|c| c.kind() == "ternary_expression");
    // If the value is a wrappable binary expression (&&, ||, or string +), skip variable
    // declarator wrapping. The binary expression's own wrapping logic will handle line-breaking.
    let value_is_wrappable_binary = children.iter().any(|c| {
        if c.kind() == "binary_expression" {
            let op = c
                .children(&mut c.walk())
                .find(|ch| !ch.is_named())
                .map(|ch| &context.source[ch.start_byte()..ch.end_byte()]);
            match op {
                Some("&&") | Some("||") => true,
                Some("+") => {
                    // Check if it's string concatenation by looking for string_literal operands
                    let mut cursor = c.walk();
                    c.children(&mut cursor)
                        .filter(|ch| ch.is_named())
                        .any(|ch| {
                            ch.kind() == "string_literal"
                                || (ch.kind() == "binary_expression" && {
                                    let inner_op = ch
                                        .children(&mut ch.walk())
                                        .find(|ic| !ic.is_named())
                                        .map(|ic| &context.source[ic.start_byte()..ic.end_byte()]);
                                    inner_op == Some("+")
                                        && ch
                                            .children(&mut ch.walk())
                                            .filter(|ic| ic.is_named())
                                            .any(|ic| ic.kind() == "string_literal")
                                })
                        })
                }
                _ => false,
            }
        } else {
            false
        }
    });
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
    let wrap_value = has_value
        && !value_is_ternary
        && !value_is_wrappable_binary
        && !value_is_array_with_comments
        && {
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
                let rhs_flat_width = expressions::collapse_whitespace(val_text).len();

                let indent_unit = context.config.indent_width as usize;
                let indent_col = context.indent_level() * indent_unit;
                // Continuation indent: current indent + 2 indent units (double indent for wrapping)
                let continuation_indent = indent_col + indent_unit * 2;
                let line_width = context.config.line_width as usize;

                // Compute LHS width: type + variable name (everything before the `=` sign).
                // We need to look at the parent node to get the type information.
                let lhs_width = if let Some(parent) = node.parent() {
                    let mut w = 0;
                    let mut cursor = parent.walk();

                    for c in parent.children(&mut cursor) {
                        // Skip until we find our variable_declarator
                        if c == node {
                            // Now add the variable_declarator's children up to the `=`
                            for vc in children.iter() {
                                if vc.kind() == "=" {
                                    break;
                                }
                                let text = &context.source[vc.start_byte()..vc.end_byte()];
                                if w > 0 {
                                    w += 1;
                                } // space between tokens
                                w += expressions::collapse_whitespace(text).len();
                            }
                            break;
                        }

                        // Accumulate width from type, modifiers, etc. before variable_declarator
                        if c.is_named() {
                            let text = &context.source[c.start_byte()..c.end_byte()];
                            if w > 0 {
                                w += 1;
                            } // space between tokens
                            w += expressions::collapse_whitespace(text).len();
                        }
                    }
                    w
                } else {
                    // Fallback: just the variable_declarator's LHS parts
                    let mut w = 0;
                    for c in children.iter() {
                        if c.kind() == "=" {
                            break;
                        }
                        let text = &context.source[c.start_byte()..c.end_byte()];
                        if w > 0 {
                            w += 1;
                        }
                        w += expressions::collapse_whitespace(text).len();
                    }
                    w
                };

                // PJF-style chain assignment: prefer wrapping at '=' over wrapping the chain.
                // Use flatten_chain to get the TRUE chain root and first segment.
                let is_chain =
                    val.kind() == "method_invocation" && expressions::chain_depth(*val) >= 1;

                if is_chain {
                    let (root_width, first_seg_width) =
                        expressions::chain_root_first_seg_width(*val, context.source);

                    // Check if `LHS = root.firstMethod()` fits on one line
                    let lhs_plus_first_seg =
                        indent_col + lhs_width + 3 + root_width + first_seg_width;

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
                        if !chain_fits_current {
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
                        } else {
                            false // Chain fits at current position, no wrapping needed
                        }
                    }
                } else {
                    // Anonymous class bodies always wrap at `=` (they're inherently multi-line)
                    let is_anonymous_class = val.kind() == "object_creation_expression" && {
                        let mut vc = val.walk();
                        val.children(&mut vc).any(|c| c.kind() == "class_body")
                    };
                    if is_anonymous_class {
                        let total_line_width =
                            indent_col + lhs_width + 3 + rhs_flat_width + 1;
                        total_line_width > line_width
                    } else {
                        // PJF-style: only break at `=` when the RHS fits on one continuation
                        // line (breakOnlyIfInnerLevelsThenFitOnOneLine). If the RHS itself
                        // is too wide, don't break at `=` — keep `= expr(` inline and let
                        // the expression's internal wrapping (arg list, etc.) handle it.
                        let rhs_fits_at_continuation =
                            continuation_indent + rhs_flat_width <= line_width;

                        let total_line_width =
                            indent_col + lhs_width + 3 + rhs_flat_width + 1;
                        let total_too_wide = total_line_width > line_width;

                        rhs_fits_at_continuation && total_too_wide
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
            "identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "dimensions" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "=" => {
                items.extend(helpers::gen_space());
                items.push_string("=".to_string());
                saw_eq = true;
                if wrap_value {
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::StartIndent);
                    items.push_signal(Signal::NewLine);
                } else {
                    items.extend(helpers::gen_space());
                }
            }
            _ if child.is_named() => {
                // If we wrapped at '=' and the RHS is a chain, tell the chain
                // wrapper that the assignment already wrapped (prefix is on prev line)
                if wrap_value && saw_eq && child.kind() == "method_invocation" {
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
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
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
pub fn gen_argument_list<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    let args: Vec<_> = children.iter().filter(|c| c.is_named()).collect();

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
                let has_block = a
                    .children(&mut cursor)
                    .any(|c| c.kind() == "block");
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
            && (p.child_by_field_name("object")
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
            .map(|n| {
                let text = &context.source[n.start_byte()..n.end_byte()];
                text.len()
            })
            .unwrap_or(0);
        let type_args_width = parent_node
            .and_then(|p| p.child_by_field_name("type_arguments"))
            .map(|ta| {
                let text = &context.source[ta.start_byte()..ta.end_byte()];
                super::expressions::collapse_whitespace(text).len()
            })
            .unwrap_or(0);
        1 + type_args_width + name_width // "." + type_args + name
    } else {
        estimate_prefix_width(node, context.source)
    };

    // Check if args fit on the same line as the prefix.
    let fits_on_one_line = if args.is_empty() {
        true
    } else if args.len() == 1 && is_in_chain {
        // For single-arg calls in chains, keep inline — the chain handles layout.
        true
    } else if args.len() == 1 && args[0].kind() == "binary_expression" {
        // Single-arg binary expressions (string concat, arithmetic, etc.) always
        // stay inline after '('. The binary expression wraps at its operators.
        true
    } else {
        indent_width + prefix_width + args_flat_width + 2 < context.config.line_width as usize
    };

    // If not, check if args fit on ONE continuation line (8-space indent = 2 levels of indent_width)
    let continuation_indent = indent_width + (2 * context.config.indent_width as usize);
    let fits_on_continuation_line =
        continuation_indent + args_flat_width + 1 < context.config.line_width as usize;

    items.push_string("(".to_string());

    if fits_on_one_line {
        // Keep all args on the same line as the opening paren
        for (i, arg) in args.iter().enumerate() {
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
            }
        }
        items.push_string(")".to_string());
    } else if fits_on_continuation_line {
        // Wrap after opening paren, but put all args on ONE continuation line (bin-packing)
        items.push_signal(Signal::StartIndent);
        items.push_signal(Signal::StartIndent);
        items.push_signal(Signal::NewLine);
        context.add_continuation_indent(2);
        for (i, arg) in args.iter().enumerate() {
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
            }
        }
        context.remove_continuation_indent(2);
        items.push_string(")".to_string());
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
    } else {
        // Args don't fit on one continuation line, put each arg on its own line
        items.push_signal(Signal::StartIndent);
        items.push_signal(Signal::StartIndent);
        context.add_continuation_indent(2);
        for (i, arg) in args.iter().enumerate() {
            items.push_signal(Signal::NewLine);
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_string(",".to_string());
            }
        }
        context.remove_continuation_indent(2);
        items.push_string(")".to_string());
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
    }

    items
}

/// Generic handler for bodies with member declarations (class_body, interface_body, etc.)
///
/// Uses dprint-core's StartIndent/FinishIndent signals so that NewLine
/// automatically gets the correct indentation. Handles comment (extra) nodes
/// that appear between members.
fn gen_body_with_members<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_string("{".to_string());

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Include both named members and extra (comment) nodes, excluding braces
    let members: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "{" && c.kind() != "}" && (c.is_named() || c.is_extra()))
        .collect();

    if members.is_empty() {
        items.push_string("}".to_string());
        return items;
    }

    items.push_signal(Signal::StartIndent);
    context.indent();

    let mut prev_was_line_comment = false;
    // Track whether previous member was a block member (has body ending with })
    let mut prev_was_block: Option<bool> = None; // None = first member after {
    let mut blank_already_inserted = false;

    // Check if a member has a block body (ends with })
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
        // method_declaration with a body (not abstract/interface methods ending with ;)
        if kind == "method_declaration" {
            return node.child_by_field_name("body").is_some();
        }
        false
    }

    for (idx, member) in members.iter().enumerate() {
        if member.is_extra() {
            let is_trailing = comments::is_trailing_comment(**member);
            if is_trailing {
                // Trailing comment: append on same line
                items.extend(helpers::gen_space());
                items.extend(gen_node(**member, context));
                prev_was_line_comment = member.kind() == "line_comment";

            } else {
                // Leading/standalone comment within body
                if !prev_was_line_comment {
                    items.push_signal(Signal::NewLine);
                }
                // Determine if we need a blank line before this comment
                if !blank_already_inserted {
                    let need_blank = match prev_was_block {
                        // First member after { — no blank
                        None => false,
                        Some(prev_block) => {
                            let next_is_block = members[idx + 1..]
                                .iter()
                                .find(|m| !m.is_extra())
                                .is_some_and(|m| is_block_member(m));
                            prev_block || next_is_block
                        }
                    };
                    if need_blank {
                        items.push_signal(Signal::NewLine);
                        blank_already_inserted = true;
                    }
                }
                items.extend(gen_node(**member, context));
                prev_was_line_comment = member.kind() == "line_comment";

                // Don't update prev_was_block — comments don't change it
            }
            continue;
        }

        if !prev_was_line_comment {
            items.push_signal(Signal::NewLine);
        }
        // Add blank line between members when either is a block member
        if !blank_already_inserted {
            let need_blank = match prev_was_block {
                // First member after { — no blank
                None => false,
                Some(prev_block) => {
                    let cur_is_block = is_block_member(member);
                    prev_block || cur_is_block
                }
            };
            if need_blank {
                items.push_signal(Signal::NewLine);
            }
        }
        items.extend(gen_node(**member, context));

        prev_was_line_comment = false;
        prev_was_block = Some(is_block_member(member));
        blank_already_inserted = false;
    }

    items.push_signal(Signal::FinishIndent);
    context.dedent();
    if !prev_was_line_comment {
        items.push_signal(Signal::NewLine);
    }
    items.push_string("}".to_string());

    items
}
