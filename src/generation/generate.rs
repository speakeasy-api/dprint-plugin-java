use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use crate::configuration::Configuration;

use super::comments;
use super::context::FormattingContext;
use super::declarations;
use super::expressions;
use super::helpers;
use super::statements;

/// Generate dprint PrintItems IR from a tree-sitter parse tree.
pub fn generate(source: &str, tree: &tree_sitter::Tree, config: &Configuration) -> PrintItems {
    let mut context = FormattingContext::new(source, config);
    let root = tree.root_node();
    gen_node(root, &mut context)
}

/// Generate PrintItems for a tree-sitter node.
///
/// This is the main dispatcher that routes nodes to specific handlers
/// based on their kind. Unhandled nodes fall back to emitting their
/// source text unchanged.
pub fn gen_node<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    context.push_parent(node.kind());
    let items = match node.kind() {
        "program" => gen_program(node, context),

        // --- Declarations ---
        "package_declaration" => declarations::gen_package_declaration(node, context),
        "import_declaration" => declarations::gen_import_declaration(node, context),
        "class_declaration" => declarations::gen_class_declaration(node, context),
        "interface_declaration" => declarations::gen_interface_declaration(node, context),
        "enum_declaration" => declarations::gen_enum_declaration(node, context),
        "record_declaration" => declarations::gen_record_declaration(node, context),
        "annotation_type_declaration" => {
            declarations::gen_annotation_type_declaration(node, context)
        }
        "method_declaration" => declarations::gen_method_declaration(node, context),
        "constructor_declaration" => declarations::gen_constructor_declaration(node, context),
        "field_declaration" | "constant_declaration" => {
            declarations::gen_field_declaration(node, context)
        }
        "class_body" | "interface_body" | "annotation_type_body" => {
            declarations::gen_class_body(node, context)
        }

        // --- Statements ---
        "block" | "constructor_body" => statements::gen_block(node, context),
        "local_variable_declaration" => statements::gen_local_variable_declaration(node, context),
        "expression_statement" => statements::gen_expression_statement(node, context),
        "if_statement" => statements::gen_if_statement(node, context),
        "for_statement" => statements::gen_for_statement(node, context),
        "enhanced_for_statement" => statements::gen_enhanced_for_statement(node, context),
        "while_statement" => statements::gen_while_statement(node, context),
        "do_statement" => statements::gen_do_statement(node, context),
        "switch_expression" => statements::gen_switch_expression(node, context),
        "try_statement" => statements::gen_try_statement(node, context),
        "try_with_resources_statement" => {
            statements::gen_try_with_resources_statement(node, context)
        }
        "return_statement" => statements::gen_return_statement(node, context),
        "throw_statement" => statements::gen_throw_statement(node, context),
        "break_statement" => statements::gen_break_statement(node, context),
        "continue_statement" => statements::gen_continue_statement(node, context),
        "yield_statement" => statements::gen_yield_statement(node, context),
        "synchronized_statement" => statements::gen_synchronized_statement(node, context),
        "assert_statement" => statements::gen_assert_statement(node, context),
        "labeled_statement" => statements::gen_labeled_statement(node, context),

        // --- Types (pass-through for now, formatted as text) ---
        "type_identifier"
        | "void_type"
        | "integral_type"
        | "floating_point_type"
        | "boolean_type"
        | "scoped_type_identifier" => helpers::gen_node_text(node, context.source),
        "generic_type" => gen_generic_type(node, context),
        "array_type" => gen_array_type(node, context),
        "type_parameter" => gen_type_parameter(node, context),
        "wildcard" => gen_wildcard(node, context),

        // --- Shared nodes ---
        "formal_parameter" | "spread_parameter" => gen_formal_parameter(node, context),
        "variable_declarator" => declarations::gen_variable_declarator(node, context),
        "argument_list" => declarations::gen_argument_list(node, context),
        "marker_annotation" => gen_marker_annotation(node, context),
        "annotation" => gen_annotation(node, context),
        "annotation_argument_list" => gen_annotation_argument_list(node, context),
        "element_value_pair" => gen_element_value_pair(node, context),
        "dimensions_expr" => gen_dimensions_expr(node, context),

        // --- Comments ---
        "line_comment" => comments::gen_line_comment(node, context),
        "block_comment" => comments::gen_block_comment(node, context),

        // --- Expressions ---
        "binary_expression" => expressions::gen_binary_expression(node, context),
        "unary_expression" => expressions::gen_unary_expression(node, context),
        "update_expression" => expressions::gen_update_expression(node, context),
        "method_invocation" => expressions::gen_method_invocation(node, context),
        "field_access" => expressions::gen_field_access(node, context),
        "lambda_expression" => expressions::gen_lambda_expression(node, context),
        "ternary_expression" => expressions::gen_ternary_expression(node, context),
        "object_creation_expression" => expressions::gen_object_creation_expression(node, context),
        "array_creation_expression" => expressions::gen_array_creation_expression(node, context),
        "array_initializer" => expressions::gen_array_initializer(node, context),
        "array_access" => expressions::gen_array_access(node, context),
        "cast_expression" => expressions::gen_cast_expression(node, context),
        "instanceof_expression" => expressions::gen_instanceof_expression(node, context),
        "parenthesized_expression" => expressions::gen_parenthesized_expression(node, context),
        "method_reference" => expressions::gen_method_reference(node, context),
        "assignment_expression" => expressions::gen_assignment_expression(node, context),
        "inferred_parameters" => expressions::gen_inferred_parameters(node, context),
        "explicit_constructor_invocation" => {
            expressions::gen_explicit_constructor_invocation(node, context)
        }

        // --- Fallback: emit source text unchanged ---
        _ => helpers::gen_node_text(node, context.source),
    };
    context.pop_parent();
    items
}

/// Generate a program node (the root of the parse tree).
fn gen_program<'a>(node: tree_sitter::Node<'a>, context: &mut FormattingContext<'a>) -> PrintItems {
    let mut items = PrintItems::new();

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // First pass: collect and categorize imports
    let mut static_imports: Vec<tree_sitter::Node> = vec![];
    let mut regular_imports: Vec<tree_sitter::Node> = vec![];
    let mut non_import_children: Vec<tree_sitter::Node> = vec![];

    for child in children.iter() {
        if child.kind() == "import_declaration" {
            let is_static = {
                let mut c = child.walk();
                child.children(&mut c).any(|ch| ch.kind() == "static")
            };

            // Extract import path to check for java.lang.* simple class imports
            let import_path = extract_import_path(*child, context.source);

            // Skip java.lang.X imports (simple class names only, not sub-packages)
            // Keep: java.lang.annotation.Retention, static imports from java.lang
            if !is_static && is_java_lang_simple_import(&import_path) {
                continue; // Skip this import
            }

            if is_static {
                static_imports.push(*child);
            } else {
                regular_imports.push(*child);
            }
        } else {
            non_import_children.push(*child);
        }
    }

    // Sort imports alphabetically by their full path
    static_imports.sort_by(|a, b| {
        let path_a = extract_import_path(*a, context.source);
        let path_b = extract_import_path(*b, context.source);
        path_a.cmp(&path_b)
    });

    regular_imports.sort_by(|a, b| {
        let path_a = extract_import_path(*a, context.source);
        let path_b = extract_import_path(*b, context.source);
        path_a.cmp(&path_b)
    });

    // Second pass: emit nodes in order
    let mut prev_kind: Option<&str> = None;
    let mut prev_was_comment = false;
    let mut emitted_imports = false;

    // Check if we have a package declaration
    let has_package = non_import_children
        .iter()
        .any(|c| c.kind() == "package_declaration");

    for (i, child) in non_import_children.iter().enumerate() {
        // Emit imports:
        // - After package declaration (if present), OR
        // - Before first non-extra node (if no package declaration)
        let should_emit_imports = !emitted_imports
            && (!static_imports.is_empty() || !regular_imports.is_empty())
            && ((has_package && prev_kind == Some("package_declaration"))
                || (!has_package && !child.is_extra()));

        if should_emit_imports {
            // Add blank line after package declaration
            if prev_kind == Some("package_declaration") {
                items.push_signal(Signal::NewLine);
            }

            // Emit static imports
            for import_node in static_imports.iter() {
                items.extend(gen_node(*import_node, context));
                items.push_signal(Signal::NewLine);
            }

            // Blank line between static and regular imports
            if !static_imports.is_empty() && !regular_imports.is_empty() {
                items.push_signal(Signal::NewLine);
            }

            // Emit regular imports
            for import_node in regular_imports.iter() {
                items.extend(gen_node(*import_node, context));
                items.push_signal(Signal::NewLine);
            }

            prev_kind = Some("import_declaration");
            prev_was_comment = false;
            emitted_imports = true;
        }

        if child.is_extra() {
            // Handle comment nodes
            let is_trailing = comments::is_trailing_comment(*child);

            if is_trailing {
                // Trailing comment: append on same line
                items.extend(helpers::gen_space());
                items.extend(gen_node(*child, context));
            } else {
                // Leading/standalone comment: emit on its own line
                if prev_kind.is_some() || prev_was_comment {
                    // Determine if we need a blank line before this comment
                    let prev_is_different_section =
                        prev_kind.is_some_and(|pk| pk != "line_comment" && pk != "block_comment");
                    let is_block_comment = child.kind() == "block_comment";

                    if prev_is_different_section && !prev_was_comment {
                        // Add blank line before comment (previous statement's newline + this newline = blank line)
                        items.push_signal(Signal::NewLine);
                        // For block comments (not line comments), add an extra newline
                        if is_block_comment {
                            items.push_signal(Signal::NewLine);
                        }
                    } else if prev_was_comment {
                        // Separate consecutive comments
                        items.push_signal(Signal::NewLine);
                    }
                    // Don't add newline here - the previous statement already ended with one
                }
                items.extend(gen_node(*child, context));
                prev_kind = Some(child.kind());
                prev_was_comment = true;
            }
            continue;
        }

        // Add blank lines between different top-level sections
        // But skip this if the current child is a comment (comments handle their own spacing)
        // Also skip if previous was a line comment (line comments are transparent for spacing)
        // Block comments still need blank lines after them
        if let Some(pk) = prev_kind
            && !child.is_extra() && pk != "line_comment" {
                let needs_double_newline = (pk == "package_declaration")
                    || pk != "import_declaration"
                    || child.kind() != "import_declaration";

                if needs_double_newline {
                    items.push_signal(Signal::NewLine);
                }
            }
        // Note: if prev_was_comment, the comment already includes a trailing newline,
        // so we don't need to add another one here

        items.extend(gen_node(*child, context));
        prev_kind = Some(child.kind());
        prev_was_comment = false;

        // Add newline after each top-level declaration
        if i < non_import_children.len() - 1
            && non_import_children[i + 1..].iter().any(|c| !c.is_extra()) {
                items.push_signal(Signal::NewLine);
            }
    }

    // Ensure file ends with a newline
    items.push_signal(Signal::NewLine);

    items
}

/// Extract the import path from an import_declaration node.
fn extract_import_path(node: tree_sitter::Node, source: &str) -> String {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
            let path = &source[child.start_byte()..child.end_byte()];
            // Include asterisk if present
            let mut next_cursor = node.walk();
            let has_asterisk = node
                .children(&mut next_cursor)
                .any(|c| c.kind() == "asterisk");
            if has_asterisk {
                return format!("{}.*", path);
            }
            return path.to_string();
        }
    }
    String::new()
}

/// Check if an import is a simple java.lang.* import that should be removed.
/// Returns true for: java.lang.String, java.lang.Override, etc.
/// Returns false for: java.lang.annotation.*, java.util.*, etc.
fn is_java_lang_simple_import(import_path: &str) -> bool {
    if let Some(rest) = import_path.strip_prefix("java.lang.") {
        // Check if there are more dots (sub-package)
        // If no more dots and not a wildcard, it's a simple class import
        !rest.contains('.') && rest != "*"
    } else {
        false
    }
}

/// Format a generic type: `List<String>`, `Map<K, V>`
fn gen_generic_type<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "type_arguments" => {
                items.extend(gen_type_arguments(child, context));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format type arguments: `<String, Integer>`
fn gen_type_arguments<'a>(
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
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format an array type: `int[]`, `String[][]`
fn gen_array_type<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "dimensions" => items.extend(helpers::gen_node_text(child, context.source)),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}

/// Format a type parameter: `T`, `T extends Comparable<T>`
fn gen_type_parameter<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "type_identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "type_bound" => {
                items.extend(helpers::gen_space());
                items.extend(gen_type_bound(child, context));
            }
            "extends" => {
                items.extend(helpers::gen_space());
                items.push_string("extends".to_string());
            }
            _ => {}
        }
    }

    items
}

/// Format a type bound: `extends Comparable<T> & Serializable`
fn gen_type_bound<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut first = true;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "extends" => {
                items.push_string("extends".to_string());
            }
            "&" => {
                items.extend(helpers::gen_space());
                items.push_string("&".to_string());
                items.extend(helpers::gen_space());
            }
            _ if child.is_named() => {
                if !first {
                    // Space already added after &
                } else {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_node(child, context));
                first = false;
            }
            _ => {}
        }
    }

    items
}

/// Format a wildcard: `?`, `? extends T`, `? super T`
fn gen_wildcard<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "?" => items.push_string("?".to_string()),
            "extends" => {
                items.extend(helpers::gen_space());
                items.push_string("extends".to_string());
            }
            "super" => {
                items.extend(helpers::gen_space());
                items.push_string("super".to_string());
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

/// Format a formal parameter: `String name`, `final int x`, `String... args`
fn gen_formal_parameter<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut need_space = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(gen_node(child, context));
                need_space = true;
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
            "..." => {
                items.push_string("...".to_string());
                need_space = true;
            }
            "identifier" | "variable_declarator" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_node(child, context));
                need_space = false;
            }
            "dimensions" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            _ => {}
        }
    }

    items
}

/// Format a marker annotation: `@Override`
fn gen_marker_annotation<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_string("@".to_string());

    if let Some(name) = node.child_by_field_name("name") {
        items.extend(helpers::gen_node_text(name, context.source));
    }

    items
}

/// Format an annotation: `@SuppressWarnings("unchecked")`
fn gen_annotation<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_string("@".to_string());

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "@" => {} // Already emitted
            "identifier" | "scoped_identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "annotation_argument_list" => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format annotation argument list: `("value")` or `(key = value)`
fn gen_annotation_argument_list<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    items.push_string("(".to_string());
    let mut first = true;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "(" | ")" => {}
            "," => {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
            }
            _ if child.is_named() => {
                if !first {
                    // Comma already handled
                }
                items.extend(gen_node(child, context));
                first = false;
            }
            _ => {}
        }
    }

    items.push_string(")".to_string());
    items
}

/// Format element value pair: `key = value`
fn gen_element_value_pair<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "=" => {
                items.extend(helpers::gen_space());
                items.push_string("=".to_string());
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

/// Format dimensions expression: `[expr]`
fn gen_dimensions_expr<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "[" => items.push_string("[".to_string()),
            "]" => items.push_string("]".to_string()),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}
