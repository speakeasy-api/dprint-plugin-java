use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use super::comments;
use super::context::FormattingContext;
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

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(gen_modifiers(child, context));
                need_space = true;
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
                items.extend(helpers::gen_space());
                items.extend(gen_superclass(child, context));
                need_space = true;
            }
            "super_interfaces" => {
                items.extend(helpers::gen_space());
                items.extend(gen_super_interfaces(child, context));
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

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(gen_modifiers(child, context));
                need_space = true;
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
                items.extend(helpers::gen_space());
                items.extend(gen_extends_interfaces(child, context));
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

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(gen_modifiers(child, context));
                need_space = true;
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
                items.extend(helpers::gen_space());
                items.extend(gen_super_interfaces(child, context));
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

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(gen_modifiers(child, context));
                need_space = true;
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
                items.extend(helpers::gen_space());
                items.extend(gen_super_interfaces(child, context));
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
                items.extend(gen_modifiers(child, context));
                need_space = true;
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
    let wrap_throws = indent_width + sig_width > context.config.line_width as usize;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(gen_modifiers(child, context));
                need_space = true;
            }
            "type_parameters" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_type_parameters(child, context));
                need_space = true;
            }
            // Return type: various type nodes
            "void_type" | "integral_type" | "floating_point_type" | "boolean_type"
            | "type_identifier" | "scoped_type_identifier" | "generic_type" | "array_type" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_node(child, context));
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
            "block" => {
                items.extend(helpers::gen_space());
                items.extend(gen_node(child, context));
                need_space = false;
            }
            ";" => {
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
                if width > 0 && child.kind() != "formal_parameters" && child.kind() != "(" && child.kind() != ")" {
                    width += 1; // space separator
                }
                width += first_line.trim().len();
            }
        }
    }

    width
}

/// Format a constructor declaration.
pub fn gen_constructor_declaration<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(gen_modifiers(child, context));
                need_space = true;
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
                items.extend(helpers::gen_space());
                items.extend(gen_throws(child, context));
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
                items.extend(gen_modifiers(child, context));
                need_space = true;
            }
            // Type nodes
            "void_type" | "integral_type" | "floating_point_type" | "boolean_type"
            | "type_identifier" | "scoped_type_identifier" | "generic_type" | "array_type" => {
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

/// Format modifiers (public, static, final, abstract, etc.)
///
/// Annotations are placed on their own line before keyword modifiers,
/// matching standard Java formatting conventions (Google/Palantir style).
pub fn gen_modifiers<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Separate annotations from keyword modifiers
    let annotations: Vec<_> = children
        .iter()
        .filter(|c| c.kind() == "marker_annotation" || c.kind() == "annotation")
        .collect();
    let keywords: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "marker_annotation" && c.kind() != "annotation")
        .collect();

    // Emit annotations, each on their own line
    for ann in &annotations {
        items.extend(gen_node(**ann, context));
        if !keywords.is_empty() || ann != annotations.last().unwrap() {
            // Newline after each annotation (before keywords or next annotation)
            items.push_signal(Signal::NewLine);
        }
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

    items
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
fn gen_class_body<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    gen_body_with_members(node, context)
}

/// Format an interface body.
fn gen_interface_body<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    gen_body_with_members(node, context)
}

/// Format an annotation type body.
fn gen_annotation_type_body<'a>(
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
    let enum_constants: Vec<_> = members.iter().filter(|c| c.kind() == "enum_constant").collect();
    let has_body_decls = members.iter().any(|c| c.kind() == "enum_body_declarations" || c.kind() == ";");

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
                // Trailing comma after each constant except the last before ";"
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
                items.extend(gen_modifiers(child, context));
                items.extend(helpers::gen_space());
            }
            "identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "argument_list" => {
                items.extend(gen_argument_list(child, context));
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
        .filter(|c| c.kind() == "formal_parameter" || c.kind() == "spread_parameter" || c.kind() == "receiver_parameter")
        .collect();

    // Calculate total inline width of params (stable: uses indent_level, not source column)
    let param_text_width: usize = params.iter().enumerate().map(|(i, p)| {
        let text = &context.source[p.start_byte()..p.end_byte()];
        let flat: usize = text.lines().map(|l| l.trim().len()).sum();
        flat + if i < params.len() - 1 { 2 } else { 0 }
    }).sum();
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let should_wrap = params.len() > 1
        && indent_width + param_text_width + 2 > context.config.line_width as usize;

    items.push_string("(".to_string());

    if should_wrap {
        // 2x StartIndent for 8-space continuation indent
        items.push_signal(Signal::StartIndent);
        items.push_signal(Signal::StartIndent);
        for (i, param) in params.iter().enumerate() {
            items.push_signal(Signal::NewLine);
            items.extend(gen_node(**param, context));
            if i < params.len() - 1 {
                items.push_string(",".to_string());
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
fn gen_throws<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "throws" => items.push_string("throws".to_string()),
            "," => {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
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
    let wrap_value = has_value && !value_is_ternary && {
        let indent_width = context.indent_level() * context.config.indent_width as usize;
        let decl_flat_width = if let Some(parent) = node.parent() {
            estimate_decl_flat_width(parent, context.source)
        } else {
            0
        };
        indent_width + decl_flat_width > context.config.line_width as usize
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
                items.extend(gen_node(child, context));
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

/// Estimate the flat width of a field/local-variable declaration as if it were
/// on a single line: `modifiers type name = value;`
///
/// Walks the declaration's children and computes widths from source text,
/// stripping leading indent from each line and joining with single spaces.
fn estimate_decl_flat_width(node: tree_sitter::Node, source: &str) -> usize {
    let mut cursor = node.walk();
    let mut width = 0;
    let mut need_space = false;

    for child in node.children(&mut cursor) {
        if child.is_extra() {
            continue;
        }
        match child.kind() {
            ";" => {
                width += 1;
            }
            "," => {
                width += 1;
                need_space = true;
            }
            _ => {
                if need_space && width > 0 {
                    width += 1;
                }
                let text = &source[child.start_byte()..child.end_byte()];
                let mut w = 0;
                for (i, line) in text.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if i > 0 && w > 0 {
                        w += 1;
                    }
                    w += trimmed.len();
                }
                width += w;
                need_space = true;
            }
        }
    }

    width
}

/// Format an argument list: `(arg1, arg2, arg3)`
///
/// Wraps with 8-space continuation indent when the argument list would
/// exceed `line_width`. Uses stable width estimation based on `context.indent_level()`
/// to avoid instability between formatting passes.
pub fn gen_argument_list<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    let args: Vec<_> = children.iter().filter(|c| c.is_named()).collect();

    // Estimate the "flat" width of arguments (stripping embedded newlines)
    let args_flat_width: usize = args.iter().enumerate().map(|(i, a)| {
        let text = &context.source[a.start_byte()..a.end_byte()];
        let flat: usize = text.lines().map(|l| l.trim().len()).sum();
        flat + if i < args.len() - 1 { 2 } else { 0 }
    }).sum();

    // Use indent level (stable across passes) + arg_list_text_width for wrap decision.
    // We don't know the exact column, so conservatively estimate with indent + some margin.
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let should_wrap = args.len() > 1
        && indent_width + args_flat_width + 2 > context.config.line_width as usize;

    items.push_string("(".to_string());

    if should_wrap {
        items.push_signal(Signal::StartIndent);
        items.push_signal(Signal::StartIndent);
        for (i, arg) in args.iter().enumerate() {
            items.push_signal(Signal::NewLine);
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_string(",".to_string());
            }
        }
        items.push_string(")".to_string());
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
    } else {
        for (i, arg) in args.iter().enumerate() {
            items.extend(gen_node(**arg, context));
            if i < args.len() - 1 {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
            }
        }
        items.push_string(")".to_string());
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

    let mut prev_kind: Option<&str> = None;
    let mut prev_was_comment = false;

    for member in &members {
        if member.is_extra() {
            let is_trailing = comments::is_trailing_comment(**member);
            if is_trailing {
                // Trailing comment: append on same line
                items.extend(helpers::gen_space());
                items.extend(gen_node(**member, context));
            } else {
                // Leading/standalone comment within body
                if prev_kind.is_some() && !prev_was_comment {
                    // Add blank line before a comment block that follows a
                    // multiline member (method, class, etc.)
                    if let Some(pk) = prev_kind {
                        if is_multiline_member(pk) {
                            items.push_signal(Signal::NewLine);
                        }
                    }
                }
                items.push_signal(Signal::NewLine);
                items.extend(gen_node(**member, context));
                prev_was_comment = true;
            }
            continue;
        }

        // Add blank line between different member types or before/after methods.
        // But NOT when a Javadoc/comment immediately precedes this member —
        // the blank line was already added before the comment.
        if let Some(pk) = prev_kind {
            if !prev_was_comment && (is_multiline_member(pk) || is_multiline_member(member.kind())) {
                items.push_signal(Signal::NewLine);
            }
        }

        items.push_signal(Signal::NewLine);
        items.extend(gen_node(**member, context));

        prev_kind = Some(member.kind());
        prev_was_comment = false;
    }

    items.push_signal(Signal::FinishIndent);
    context.dedent();
    items.push_signal(Signal::NewLine);
    items.push_string("}".to_string());

    items
}

/// Returns true for member kinds that should have blank lines around them.
fn is_multiline_member(kind: &str) -> bool {
    matches!(
        kind,
        "method_declaration"
            | "constructor_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "static_initializer"
            | "block"
    )
}
