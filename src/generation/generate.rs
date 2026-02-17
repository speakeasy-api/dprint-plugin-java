use dprint_core::formatting::PrintItems;

use crate::configuration::Configuration;

use super::comments;
use super::context::FormattingContext;
use super::declarations;
use super::expressions;
use super::helpers::{PrintItemsExt, collapse_whitespace_len, gen_node_text, is_type_node};
use super::statements;

/// Generate dprint `PrintItems` IR from a tree-sitter parse tree.
#[must_use]
pub fn generate(source: &str, tree: &tree_sitter::Tree, config: &Configuration) -> PrintItems {
    let mut context = FormattingContext::new(source, config);
    let root = tree.root_node();
    gen_node(root, &mut context)
}

/// Generate `PrintItems` for a tree-sitter node.
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

        // --- Types ---
        "generic_type" => gen_generic_type(node, context),
        "array_type" => gen_array_type(node, context),
        kind if is_type_node(kind) => gen_node_text(node, context.source),
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
        "array_initializer" | "element_value_array_initializer" => {
            expressions::gen_array_initializer(node, context)
        }
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

        // Static initializer: `static { ... }`
        "static_initializer" => {
            let mut items = PrintItems::new();
            items.push_str("static");
            for child in node.children(&mut node.walk()) {
                if child.kind() == "block" {
                    items.space();
                    items.extend(statements::gen_block(child, context));
                }
            }
            items
        }

        // --- Fallback: emit source text unchanged ---
        _ => gen_node_text(node, context.source),
    };
    context.pop_parent();
    items
}

/// Generate a program node (the root of the parse tree).
#[allow(clippy::too_many_lines)]
fn gen_program<'a>(node: tree_sitter::Node<'a>, context: &mut FormattingContext<'a>) -> PrintItems {
    let mut items = PrintItems::new();

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // First pass: collect and categorize imports
    let mut static_imports: Vec<tree_sitter::Node> = vec![];
    let mut regular_imports: Vec<tree_sitter::Node> = vec![];
    let mut non_import_children: Vec<tree_sitter::Node> = vec![];

    for child in &children {
        if child.kind() == "import_declaration" {
            let is_static = {
                let mut c = child.walk();
                child.children(&mut c).any(|ch| ch.kind() == "static")
            };

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
    let mut prev_end_row: Option<usize> = None;
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
                items.newline();
            }

            // Emit static imports
            for import_node in &static_imports {
                items.extend(gen_node(*import_node, context));
                items.newline();
            }

            // Blank line between static and regular imports
            if !static_imports.is_empty() && !regular_imports.is_empty() {
                items.newline();
            }

            // Emit regular imports
            for import_node in &regular_imports {
                items.extend(gen_node(*import_node, context));
                items.newline();
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
                items.space();
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
                        // Exception: after imports, we only add ONE blank line total (not two)
                        if (prev_kind == Some("import_declaration")
                            || prev_kind == Some("package_declaration"))
                            && is_block_comment
                        {
                            // Add one newline to create the blank line (import already has its newline)
                            items.newline();
                        } else {
                            items.newline();
                            // For block comments (not line comments), add an extra newline
                            if is_block_comment {
                                items.newline();
                            }
                        }
                    } else if prev_was_comment && child.kind() != "line_comment" {
                        // Separate consecutive block comments with blank line.
                        // Consecutive line comments stay tightly grouped.
                        items.newline();
                    } else if prev_was_comment && child.kind() == "line_comment" {
                        if prev_kind == Some("block_comment") {
                            // Block comments don't emit trailing newlines, so we always
                            // need at least one newline before the next line comment.
                            items.newline();
                        }
                        if prev_end_row.is_some_and(|r| child.start_position().row > r + 1) {
                            // Source had a blank line between consecutive line comments — preserve it.
                            items.newline();
                        }
                    }
                    // Don't add newline here - the previous statement already ended with one
                }
                items.extend(gen_node(*child, context));
                prev_kind = Some(child.kind());
                prev_was_comment = true;
                prev_end_row = Some(child.end_position().row);
            }
            continue;
        }

        // Do not preserve blank lines between a header comment and package declaration.
        // palantir-java-format always removes that extra blank line.

        // Add blank lines between different top-level sections
        // But skip this if the current child is a comment (comments handle their own spacing)
        // Also skip if previous was a line comment (line comments are transparent for spacing)
        // Block comments still need blank lines after them
        // Special case: after imports, we only add ONE blank line (not two)
        if let Some(pk) = prev_kind
            && !child.is_extra()
        {
            if pk == "line_comment" {
                // After line comment: the comment already emitted a trailing newline.
                // Only add a blank if source has one.
                if prev_end_row.is_some_and(|r| child.start_position().row > r + 1) {
                    items.newline();
                }
            } else if pk == "block_comment" {
                // After block comment: block comments don't emit trailing newlines,
                // so we always need at least one newline. Add an extra if source has a blank.
                items.newline();
                if prev_end_row.is_some_and(|r| child.start_position().row > r + 1) {
                    items.newline();
                }
            } else {
                let needs_double_newline = (pk == "package_declaration")
                    || pk != "import_declaration"
                    || child.kind() != "import_declaration";

                if needs_double_newline {
                    items.newline();
                }
            }
        }

        items.extend(gen_node(*child, context));
        prev_kind = Some(child.kind());
        prev_was_comment = false;
        prev_end_row = Some(child.end_position().row);

        // Add newline after each top-level declaration
        if i < non_import_children.len() - 1
            && non_import_children[i + 1..].iter().any(|c| !c.is_extra())
        {
            items.newline();
        }
    }

    // Ensure file ends with a newline
    items.newline();

    items
}

/// Extract the import path from an `import_declaration` node.
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
                return format!("{path}.*");
            }
            return path.to_string();
        }
    }
    String::new()
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

/// Estimate the prefix width before a type arguments node, including
/// declaration modifiers or `new` where applicable. Uses collapsed
/// whitespace on the source's last line to keep estimates stable.
fn estimate_type_args_prefix_width(node: tree_sitter::Node, source: &str) -> usize {
    let Some(parent) = node.parent() else {
        return 0;
    };

    let prefix_text = &source[parent.start_byte()..node.start_byte()];
    let last_line = prefix_text.lines().last().unwrap_or(prefix_text);
    let mut width = collapse_prefix_len(last_line);

    let mut prev = parent;
    let mut ancestor = parent.parent();
    while let Some(anc) = ancestor {
        match anc.kind() {
            "method_declaration"
            | "field_declaration"
            | "local_variable_declaration"
            | "formal_parameter"
            | "object_creation_expression"
            | "method_invocation"
            | "constructor_declaration" => {
                let text = &source[anc.start_byte()..prev.start_byte()];
                let last = text.lines().last().unwrap_or(text);
                width += collapse_prefix_len(last);
                break;
            }
            "return_statement" => {
                width += 7; // "return "
                break;
            }
            "throw_statement" => {
                width += 6; // "throw "
                break;
            }
            _ => {
                prev = anc;
                ancestor = anc.parent();
            }
        }
    }

    width
}

/// Collapse whitespace for a prefix segment, preserving a trailing space
/// when the segment ends with whitespace (to account for token separators).
fn collapse_prefix_len(s: &str) -> usize {
    let trimmed_start = s.trim_start();
    if trimmed_start.is_empty() {
        return 0;
    }
    let mut len = collapse_whitespace_len(trimmed_start);
    if trimmed_start.ends_with(char::is_whitespace) {
        len += 1;
    }
    len
}

/// Format type arguments: `<String, Integer>`
///
/// When type arguments are too long, wraps each on its own line at double
/// continuation indent (PJF style):
/// ```java
/// AsyncRequestOperation<
///         BinaryAndStringUploadRequest,
///         org.openapis.review.openapi.models.operations.async.BinaryAndStringUploadResponse>
/// ```
#[allow(clippy::too_many_lines)]
fn gen_type_arguments<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Collect type argument nodes
    let type_args: Vec<_> = children.iter().filter(|c| c.is_named()).collect();

    // Estimate flat width of the entire type_arguments including angle brackets
    let args_flat_width: usize = type_args
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let text = &context.source[a.start_byte()..a.end_byte()];
            let flat = collapse_whitespace_len(text);
            flat + if i < type_args.len() - 1 { 2 } else { 0 } // ", " between args
        })
        .sum();

    // Estimate prefix width: everything on the current line before the `<`.
    // Walk up the tree to find the full prefix including keywords like `implements`.
    // Also detect if we're in a class declaration context (followed by ` {`).
    let (base_prefix_width, in_class_decl) = {
        let parent = node.parent();
        if let Some(p) = parent {
            let mut line_start = p;
            let mut n = p;
            let mut found_clause = false;
            while let Some(par) = n.parent() {
                match par.kind() {
                    "superclass" | "super_interfaces" | "extends_interfaces" => {
                        line_start = par;
                        found_clause = true;
                        break;
                    }
                    "class_declaration"
                    | "interface_declaration"
                    | "enum_declaration"
                    | "record_declaration" => break,
                    _ => {
                        n = par;
                    }
                }
            }
            let prefix_text = &context.source[line_start.start_byte()..node.start_byte()];
            let last_line = prefix_text.lines().last().unwrap_or(prefix_text);
            (last_line.trim_start().len(), found_clause)
        } else {
            (0, false)
        }
    };

    let prefix_width = if in_class_decl {
        base_prefix_width
    } else {
        let expanded = estimate_type_args_prefix_width(node, context.source);
        base_prefix_width.max(expanded)
    };

    let indent_width = context.effective_indent_level() * context.config.indent_width as usize;
    let line_width = context.config.line_width as usize;

    // Check if type args fit inline: prefix + <args> must fit on line.
    // Add 2 for trailing " {" when in extends/implements context.
    let trailing = if in_class_decl { 2 } else { 0 };
    let total_inline = indent_width + prefix_width + 1 + args_flat_width + 1 + trailing; // <args> [+ " {"]
    let should_wrap = total_inline > line_width;

    if should_wrap {
        context.mark_type_args_wrapped();
        // PJF uses double continuation indent (+16 = 4 indent levels) for type args
        // in local variable declarations, but single continuation (+8 = 2 indent levels)
        // in class declaration contexts (extends/implements clauses).
        let indent_levels = if in_class_decl { 2 } else { 4 };
        let continuation_col = indent_width + indent_levels * context.config.indent_width as usize;
        let all_fit_continuation = continuation_col + args_flat_width + 1 + trailing <= line_width; // args + ">" [+ " {"]

        items.push_str("<");
        for _ in 0..indent_levels {
            items.start_indent();
        }

        if all_fit_continuation {
            // All type args on one continuation line
            items.newline();
            for (i, arg) in type_args.iter().enumerate() {
                items.extend(gen_node(**arg, context));
                if i < type_args.len() - 1 {
                    items.push_str(",");
                    items.space();
                }
            }
        } else {
            // One per line
            for (i, arg) in type_args.iter().enumerate() {
                items.newline();
                items.extend(gen_node(**arg, context));
                if i < type_args.len() - 1 {
                    items.push_str(",");
                }
            }
        }
        items.push_str(">");
        for _ in 0..indent_levels {
            items.finish_indent();
        }
    } else {
        for child in &children {
            match child.kind() {
                "<" => items.push_str("<"),
                ">" => items.push_str(">"),
                "," => {
                    items.push_str(",");
                    items.space();
                }
                _ if child.is_named() => {
                    items.extend(gen_node(*child, context));
                }
                _ => {}
            }
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
            "dimensions" => items.extend(gen_node_text(child, context.source)),
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
                items.extend(gen_node_text(child, context.source));
            }
            "type_bound" => {
                items.space();
                items.extend(gen_type_bound(child, context));
            }
            "extends" => {
                items.space();
                items.push_str("extends");
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
                items.push_str("extends");
            }
            "&" => {
                items.space();
                items.push_str("&");
                items.space();
            }
            _ if child.is_named() => {
                if first {
                    items.space();
                } else {
                    // Space already added after &
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
            "?" => items.push_str("?"),
            "extends" => {
                items.space();
                items.push_str("extends");
            }
            "super" => {
                items.space();
                items.push_str("super");
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
                    items.space();
                }
                items.extend(gen_node(child, context));
                need_space = true;
            }
            "..." => {
                items.push_str("...");
                need_space = true;
            }
            "identifier" | "variable_declarator" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node(child, context));
                need_space = false;
            }
            "dimensions" => {
                items.extend(gen_node_text(child, context.source));
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
    items.push_str("@");

    if let Some(name) = node.child_by_field_name("name") {
        items.extend(gen_node_text(name, context.source));
    }

    items
}

/// Format an annotation: `@SuppressWarnings("unchecked")`
fn gen_annotation<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("@");

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "scoped_identifier" => {
                items.extend(gen_node_text(child, context.source));
            }
            "annotation_argument_list" => {
                items.extend(gen_node(child, context));
            }
            // "@" already emitted above
            _ => {}
        }
    }

    items
}

/// Format annotation argument list: `("value")` or `(key = value)`
///
/// When any argument contains an `element_value_array_initializer`, forces all
/// arguments to separate lines with continuation indent (+8), matching PJF behavior.
fn gen_annotation_argument_list<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    // Check if any argument contains a multi-element array initializer.
    // A single-element array (e.g., @SuppressWarnings({"unchecked"})) stays compact.
    let has_multi_element_array = node.children(&mut cursor).any(|child| {
        // Find an element_value_array_initializer either as the child itself
        // or as a grandchild (inside element_value_pair)
        let arr_node = if child.kind() == "element_value_array_initializer" {
            Some(child)
        } else if child.kind() == "element_value_pair" {
            let mut c = child.walk();
            child
                .children(&mut c)
                .find(|gc| gc.kind() == "element_value_array_initializer")
        } else {
            None
        };
        if let Some(arr) = arr_node {
            let mut ac = arr.walk();
            let element_count = arr
                .children(&mut ac)
                .filter(tree_sitter::Node::is_named)
                .count();
            element_count > 1
        } else {
            false
        }
    });

    // Reset cursor
    cursor = node.walk();

    // Compute flat width of the entire annotation argument list
    let text = &context.source[node.start_byte()..node.end_byte()];
    let flat_width = collapse_whitespace_len(text);

    // Also need the annotation name width (go up to parent annotation node)
    let annotation_prefix_width = if let Some(parent) = node.parent() {
        let prefix = &context.source[parent.start_byte()..node.start_byte()];
        prefix.len() // e.g., "@Target" = 7 chars
    } else {
        0
    };

    let indent_col = context.indent_level() * context.config.indent_width as usize;
    let annotation_total_width = indent_col + annotation_prefix_width + flat_width;
    let exceeds_line_width = annotation_total_width > context.config.line_width as usize;

    // Force multi-line when:
    // 1. Annotation has multi-element arrays (PJF always wraps these), OR
    // 2. Annotation wouldn't fit on one line (PJF wraps long annotations one-per-line)
    // But only if there are multiple arguments (single-arg annotations stay inline)
    let named_arg_count = {
        let mut c = node.walk();
        node.children(&mut c)
            .filter(tree_sitter::Node::is_named)
            .count()
    };
    let force_multiline = (named_arg_count > 1 || has_multi_element_array) && exceeds_line_width;

    if force_multiline {
        // Multi-line format: force all args to separate lines with continuation indent (+8)
        items.push_str("(");
        // Double indent = +8 (continuation indent)
        items.start_indent();
        items.start_indent();

        let named_children: Vec<_> = node
            .children(&mut cursor)
            .filter(tree_sitter::Node::is_named)
            .collect();
        let count = named_children.len();

        for (i, child) in named_children.iter().enumerate() {
            items.newline();
            items.extend(gen_node(*child, context));
            if i < count - 1 {
                items.push_str(",");
            }
        }

        items.push_str(")");
        items.finish_indent();
        items.finish_indent();
    } else {
        // Inline format
        items.push_str("(");
        let mut first = true;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "(" | ")" => {}
                "," => {
                    items.push_str(",");
                    items.space();
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

        items.push_str(")");
    }

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
                items.extend(gen_node_text(child, context.source));
            }
            "=" => {
                items.space();
                items.push_str("=");
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

/// Format dimensions expression: `[expr]`
fn gen_dimensions_expr<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "[" => items.push_str("["),
            "]" => items.push_str("]"),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}
