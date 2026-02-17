use dprint_core::formatting::PrintItems;

use super::comments;
use super::context::FormattingContext;
use super::declarations;
use super::generate::gen_node;
use super::helpers::{PrintItemsExt, gen_node_text, is_type_node};

/// Format a block: `{ statement1; statement2; }`
///
/// Handles comment (extra) nodes that appear within the block,
/// emitting trailing comments on the same line and leading comments
/// on their own lines.
pub fn gen_block<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("{");

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Include both named statements and extra (comment) nodes
    let stmts: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "{" && c.kind() != "}" && (c.is_named() || c.is_extra()))
        .collect();

    if stmts.is_empty() {
        items.push_str("}");
        return items;
    }

    items.start_indent();
    context.indent();

    let mut prev_was_line_comment = false;
    // Initialize to opening brace's row to preserve blank lines after `{`
    let open_brace_row = children
        .iter()
        .find(|c| c.kind() == "{")
        .map(|c| c.end_position().row);
    let mut prev_end_row: Option<usize> = open_brace_row;
    for stmt in &stmts {
        if stmt.is_extra() {
            let is_trailing = comments::is_trailing_comment(**stmt);
            if is_trailing {
                // Trailing comment: append on same line
                items.space();
                items.extend(gen_node(**stmt, context));
                prev_was_line_comment = stmt.kind() == "line_comment";
                prev_end_row = Some(stmt.end_position().row);
            } else {
                // Leading/standalone comment
                if !prev_was_line_comment {
                    items.newline();
                }
                // Preserve blank line from source before this comment
                if let Some(prev_row) = prev_end_row
                    && stmt.start_position().row > prev_row + 1
                {
                    items.newline();
                }
                items.extend(gen_node(**stmt, context));
                prev_was_line_comment = stmt.kind() == "line_comment";
                prev_end_row = Some(stmt.end_position().row);
            }
            continue;
        }

        if !prev_was_line_comment {
            items.newline();
        }
        // Preserve blank line from source between statements
        if let Some(prev_row) = prev_end_row
            && stmt.start_position().row > prev_row + 1
        {
            items.newline();
        }
        items.extend(gen_node(**stmt, context));
        prev_was_line_comment = false;
        prev_end_row = Some(stmt.end_position().row);
    }

    items.finish_indent();
    context.dedent();
    // Don't emit extra newline if last item was a line comment (which already
    // includes a trailing newline), to avoid blank line before `}`.
    if !prev_was_line_comment {
        items.newline();
    }
    // PJF strips blank lines before closing `}` in method/constructor bodies
    // but preserves them in other blocks (try, if, for, etc.)
    let parent_kind = node.parent().map_or("", |p| p.kind());
    let strip_trailing_blank = matches!(
        parent_kind,
        "method_declaration" | "constructor_declaration" | "static_initializer"
    );
    if !strip_trailing_blank && let Some(prev_row) = prev_end_row {
        let close_brace_row = children
            .iter()
            .rev()
            .find(|c| c.kind() == "}")
            .map(|c| c.start_position().row);
        if let Some(close_row) = close_brace_row
            && close_row > prev_row + 1
        {
            items.newline();
        }
    }
    items.push_str("}");

    items
}

/// Format a local variable declaration: `int x = 5;`
pub fn gen_local_variable_declaration<'a>(
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
                let (modifier_items, ends_with_newline) =
                    declarations::gen_modifiers(child, context);
                items.extend(modifier_items);
                // Only need space if modifiers didn't end with newline
                need_space = !ends_with_newline;
            }
            // Type nodes
            kind if is_type_node(kind) || kind == "var" => {
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
                    items.extend(gen_node(child, context));
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
                    items.extend(gen_node(child, context));
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

/// Format an expression statement: `expr;`
pub fn gen_expression_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            ";" => items.push_str(";"),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}

/// Format an if statement: `if (cond) { } else if (cond) { } else { }`
pub fn gen_if_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    let mut i = 0;
    let mut prev_was_block = false;
    while i < children.len() {
        let child = children[i];
        match child.kind() {
            "if" => {
                items.push_str("if");
                items.space();
            }
            "parenthesized_expression" | "condition" => {
                items.extend(gen_node(child, context));
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
                prev_was_block = true;
            }
            "else" => {
                if prev_was_block {
                    // After block: `} else` on same line
                    items.space();
                } else {
                    // After brace-less statement: `else` on new line
                    items.newline();
                }
                items.push_str("else");
                items.space();
                prev_was_block = false;
            }
            "if_statement" => {
                // else if: recursively format
                items.extend(gen_if_statement(child, context));
            }
            _ if child.is_named() => {
                // Non-block consequence (single statement)
                items.extend(gen_node(child, context));
                prev_was_block = false;
            }
            _ => {}
        }
        i += 1;
    }

    items
}

/// Format a for statement: `for (init; cond; update) { }`
pub fn gen_for_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("for");
    items.space();
    items.push_str("(");

    // Use field-based access for cleaner for-statement formatting
    if let Some(init) = node.child_by_field_name("init") {
        items.extend(gen_node(init, context));
    }
    // The init (local_variable_declaration) includes its own ";"
    // but we need a space after it
    items.space();

    if let Some(condition) = node.child_by_field_name("condition") {
        items.extend(gen_node(condition, context));
    }
    items.push_str(";");
    items.space();

    if let Some(update) = node.child_by_field_name("update") {
        items.extend(gen_node(update, context));
    }
    items.push_str(")");

    if let Some(body) = node.child_by_field_name("body") {
        items.space();
        items.extend(gen_node(body, context));
    }

    items
}

/// Format an enhanced for statement: `for (Type item : collection) { }`
pub fn gen_enhanced_for_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("for");
    items.space();
    items.push_str("(");

    let mut cursor = node.walk();
    let mut need_space = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "for" | "(" | ")" => {}
            "modifiers" => {
                items.extend(gen_node(child, context));
                need_space = true;
            }
            // Type nodes
            kind if is_type_node(kind) => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node(child, context));
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
                need_space = true;
            }
            ":" => {
                items.space();
                items.push_str(":");
                items.space();
                need_space = false;
            }
            "block" => {
                items.push_str(")");
                items.space();
                items.extend(gen_block(child, context));
                return items;
            }
            _ if child.is_named() => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node(child, context));
                need_space = true;
            }
            _ => {}
        }
    }

    items.push_str(")");
    items
}

/// Format a while statement: `while (cond) { }`
pub fn gen_while_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "while" => {
                items.push_str("while");
                items.space();
            }
            "parenthesized_expression" | "condition" => {
                items.extend(gen_node(child, context));
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a do-while statement: `do { } while (cond);`
pub fn gen_do_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "do" => {
                items.push_str("do");
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            "while" => {
                items.space();
                items.push_str("while");
                items.space();
            }
            "parenthesized_expression" => {
                items.extend(gen_node(child, context));
            }
            ";" => {
                items.push_str(";");
            }
            _ => {}
        }
    }

    items
}

/// Format a switch expression/statement.
pub fn gen_switch_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "switch" => {
                items.push_str("switch");
                items.space();
            }
            "parenthesized_expression" => {
                items.extend(gen_node(child, context));
                items.space();
            }
            "switch_block" => {
                items.extend(gen_switch_block(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a switch block: `{ case X: ... }`
fn gen_switch_block<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("{");

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    let cases: Vec<_> = children.iter().filter(|c| c.is_named()).collect();

    if cases.is_empty() {
        items.push_str("}");
        return items;
    }

    items.start_indent();

    let mut prev_case_end_row: Option<usize> = children
        .iter()
        .find(|c| c.kind() == "{")
        .map(|c| c.end_position().row);
    for case in &cases {
        items.newline();
        // Preserve source blank lines between switch cases
        if let Some(prev_row) = prev_case_end_row
            && case.start_position().row > prev_row + 1
        {
            items.newline();
        }
        items.extend(gen_switch_case(**case, context));
        prev_case_end_row = Some(case.end_position().row);
    }

    items.finish_indent();
    items.newline();
    items.push_str("}");

    items
}

/// Format a switch case or switch rule.
fn gen_switch_case<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    match node.kind() {
        "switch_block_statement_group" => {
            // Traditional case: `case X: stmt1; stmt2;`
            let mut label_done = false;
            let mut in_body = false;

            // Collect body statements (named children after the colon)
            let body_stmts: Vec<_> = children
                .iter()
                .skip_while(|c| c.kind() != ":")
                .skip(1) // skip the colon itself
                .filter(|c| c.is_named())
                .collect();

            // Check if the body is a single block
            let is_single_block = body_stmts.len() == 1 && body_stmts[0].kind() == "block";

            let mut prev_stmt_end_row: Option<usize> = None;
            for child in &children {
                if child.kind() == "switch_label" {
                    if label_done {
                        items.newline();
                    }
                    items.extend(gen_switch_label(*child, context));
                    label_done = true;
                } else if child.kind() == ":" {
                    // Colon is a child of switch_block_statement_group, not switch_label
                    items.push_str(":");
                    // If the body is a single block, add a space (brace goes on same line)
                    if is_single_block {
                        items.space();
                    }
                    prev_stmt_end_row = Some(child.end_position().row);
                } else if child.is_named() {
                    if !is_single_block {
                        // Multiple statements or non-block: indent and place on new lines
                        if !in_body {
                            items.start_indent();
                            in_body = true;
                        }
                        items.newline();
                        // Preserve source blank lines between statements in case body
                        if let Some(prev_row) = prev_stmt_end_row
                            && child.start_position().row > prev_row + 1
                        {
                            items.newline();
                        }
                    }
                    items.extend(gen_node(*child, context));
                    prev_stmt_end_row = Some(child.end_position().row);
                }
            }
            if in_body {
                items.finish_indent();
            }
        }
        "switch_rule" => {
            // Arrow case: `case X -> expr;` or `case X -> { block }`
            for child in &children {
                match child.kind() {
                    "switch_label" => {
                        items.extend(gen_switch_label(*child, context));
                    }
                    "->" => {
                        items.space();
                        items.push_str("->");
                        items.space();
                    }
                    _ if child.is_named() => {
                        items.extend(gen_node(*child, context));
                    }
                    _ => {}
                }
            }
        }
        _ => {
            items.extend(gen_node_text(node, context.source));
        }
    }

    items
}

/// Format a switch label: `case X:` or `default:`
fn gen_switch_label<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "case" => {
                items.push_str("case");
                items.space();
            }
            "default" => {
                items.push_str("default");
            }
            ":" => {
                items.push_str(":");
            }
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

/// Format a try statement: `try { } catch (Exception e) { } finally { }`
pub fn gen_try_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "try" => {
                items.push_str("try");
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            "catch_clause" => {
                items.space();
                items.extend(gen_catch_clause(child, context));
            }
            "finally_clause" => {
                items.space();
                items.extend(gen_finally_clause(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a try-with-resources statement.
pub fn gen_try_with_resources_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "try" => {
                items.push_str("try");
                items.space();
            }
            "resource_specification" => {
                items.extend(gen_resource_specification(child, context));
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            "catch_clause" => {
                items.space();
                items.extend(gen_catch_clause(child, context));
            }
            "finally_clause" => {
                items.space();
                items.extend(gen_finally_clause(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Estimate the flat width of a catch clause from the source text.
/// Returns the width of `} catch (ExType1 | ExType2 ... e) {` on one line.
fn estimate_catch_clause_width(node: tree_sitter::Node, source: &str) -> usize {
    // We need to estimate: "} catch (" + types + " " + identifier + ") {"
    let mut width = "} catch (".len();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "catch_formal_parameter" {
            let text = &source[child.start_byte()..child.end_byte()];
            // Collapse all whitespace to single spaces for flat width
            let flat_text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
            width += flat_text.len();
        }
    }

    width += ") {".len();
    width
}

/// Format a catch clause: `catch (Exception e) { }`
fn gen_catch_clause<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    // Pre-calculate: estimate catch clause line width to decide multi-exception wrapping
    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let catch_width = estimate_catch_clause_width(node, context.source);
    let should_wrap_catch = indent_width + catch_width > context.config.line_width as usize;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "catch" => {
                items.push_str("catch");
                items.space();
            }
            "catch_formal_parameter" => {
                items.push_str("(");
                items.extend(gen_catch_formal_parameter(
                    child,
                    context,
                    should_wrap_catch,
                ));
                items.push_str(")");
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a catch formal parameter: `Exception | RuntimeException e`
fn gen_catch_formal_parameter<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
    should_wrap: bool,
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
            "catch_type" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_catch_type(child, context, should_wrap));
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.space();
                }
                items.extend(gen_node_text(child, context.source));
            }
            _ => {}
        }
    }

    items
}

/// Format a catch type: `Exception | RuntimeException`
/// If `should_wrap` is true, wraps at `|` separators with continuation indent.
fn gen_catch_type<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
    should_wrap: bool,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    if should_wrap {
        // Long catch: wrap at | with continuation indent
        // Tree structure: Type1 | Type2 | Type3
        // We want: Type1 on same line, then newline + | Type2, newline + | Type3, etc.

        // Add continuation indent (+2 levels = +8 spaces)
        items.start_indent();
        items.start_indent();

        for child in children {
            match child.kind() {
                "|" => {
                    // For all | tokens, emit newline + | + space
                    items.newline();
                    items.push_str("|");
                    items.space();
                }
                _ if child.is_named() => {
                    // Emit the type
                    items.extend(gen_node(child, context));
                }
                _ => {}
            }
        }

        items.finish_indent();
        items.finish_indent();
    } else {
        // Short catch: keep on one line
        for child in children {
            match child.kind() {
                "|" => {
                    items.space();
                    items.push_str("|");
                    items.space();
                }
                _ if child.is_named() => {
                    items.extend(gen_node(child, context));
                }
                _ => {}
            }
        }
    }

    items
}

/// Format a finally clause: `finally { }`
fn gen_finally_clause<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "finally" => {
                items.push_str("finally");
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format resource specification: `(Resource r = new Resource())`
fn gen_resource_specification<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    items.push_str("(");

    for child in node.children(&mut cursor) {
        match child.kind() {
            "(" | ")" => {}
            ";" => {
                items.push_str(";");
                items.space();
            }
            "resource" => {
                items.extend(gen_node(child, context));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items.push_str(")");
    items
}

/// Format a return statement: `return expr;`
pub fn gen_return_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("return");

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "return" => {}
            ";" => items.push_str(";"),
            _ if child.is_named() => {
                items.space();
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a throw statement: `throw expr;`
pub fn gen_throw_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("throw");

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "throw" => {}
            ";" => items.push_str(";"),
            _ if child.is_named() => {
                items.space();
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a break statement: `break;` or `break label;`
pub fn gen_break_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("break");

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            ";" => items.push_str(";"),
            "identifier" => {
                items.space();
                items.extend(gen_node_text(child, context.source));
            }
            // "break" keyword already emitted above
            _ => {}
        }
    }

    items
}

/// Format a continue statement: `continue;` or `continue label;`
pub fn gen_continue_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("continue");

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            ";" => items.push_str(";"),
            "identifier" => {
                items.space();
                items.extend(gen_node_text(child, context.source));
            }
            // "continue" keyword already emitted above
            _ => {}
        }
    }

    items
}

/// Format a yield statement: `yield expr;`
pub fn gen_yield_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("yield");

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "yield" => {}
            ";" => items.push_str(";"),
            _ if child.is_named() => {
                items.space();
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a synchronized statement: `synchronized (obj) { }`
pub fn gen_synchronized_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "synchronized" => {
                items.push_str("synchronized");
                items.space();
            }
            "parenthesized_expression" => {
                items.extend(gen_node(child, context));
                items.space();
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format an assert statement: `assert cond;` or `assert cond : message;`
pub fn gen_assert_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_str("assert");

    let mut cursor = node.walk();
    let mut after_colon = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "assert" => {}
            ":" => {
                items.space();
                items.push_str(":");
                after_colon = true;
            }
            ";" => items.push_str(";"),
            _ if child.is_named() => {
                items.space();
                items.extend(gen_node(child, context));
                let _ = after_colon;
            }
            _ => {}
        }
    }

    items
}

/// Format a labeled statement: `label: statement`
pub fn gen_labeled_statement<'a>(
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
            ":" => {
                items.push_str(":");
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
