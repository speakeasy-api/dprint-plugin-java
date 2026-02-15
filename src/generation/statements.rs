use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use super::comments;
use super::context::FormattingContext;
use super::declarations;
use super::generate::gen_node;
use super::helpers;

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
    items.push_string("{".to_string());

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Include both named statements and extra (comment) nodes
    let stmts: Vec<_> = children
        .iter()
        .filter(|c| c.kind() != "{" && c.kind() != "}" && (c.is_named() || c.is_extra()))
        .collect();

    if stmts.is_empty() {
        items.push_string("}".to_string());
        return items;
    }

    items.push_signal(Signal::StartIndent);
    context.indent();

    let mut prev_was_line_comment = false;
    for stmt in &stmts {
        if stmt.is_extra() {
            let is_trailing = comments::is_trailing_comment(**stmt);
            if is_trailing {
                // Trailing comment: append on same line
                items.extend(helpers::gen_space());
                items.extend(gen_node(**stmt, context));
                prev_was_line_comment = stmt.kind() == "line_comment";
            } else {
                // Leading/standalone comment
                if !prev_was_line_comment {
                    items.push_signal(Signal::NewLine);
                }
                items.extend(gen_node(**stmt, context));
                prev_was_line_comment = stmt.kind() == "line_comment";
            }
            continue;
        }

        if !prev_was_line_comment {
            items.push_signal(Signal::NewLine);
        }
        items.extend(gen_node(**stmt, context));
        prev_was_line_comment = false;
    }

    items.push_signal(Signal::FinishIndent);
    context.dedent();
    items.push_signal(Signal::NewLine);
    items.push_string("}".to_string());

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

    for child in node.children(&mut cursor) {
        match child.kind() {
            "modifiers" => {
                items.extend(declarations::gen_modifiers(child, context));
                let mut mc = child.walk();
                let has_ann = child.children(&mut mc).any(|c| {
                    c.kind() == "marker_annotation" || c.kind() == "annotation"
                });
                let mut mc2 = child.walk();
                let has_kw = child.children(&mut mc2).any(|c| {
                    c.kind() != "marker_annotation" && c.kind() != "annotation"
                });
                if has_ann && !has_kw {
                    items.push_signal(Signal::NewLine);
                    need_space = false;
                } else {
                    need_space = true;
                }
            }
            // Type nodes
            "void_type" | "integral_type" | "floating_point_type" | "boolean_type"
            | "type_identifier" | "scoped_type_identifier" | "generic_type" | "array_type"
            | "var" => {
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
                items.extend(gen_node(child, context));
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

/// Format an expression statement: `expr;`
pub fn gen_expression_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            ";" => items.push_string(";".to_string()),
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
    while i < children.len() {
        let child = children[i];
        match child.kind() {
            "if" => {
                items.push_string("if".to_string());
                items.extend(helpers::gen_space());
            }
            "parenthesized_expression" | "condition" => {
                items.extend(gen_node(child, context));
                items.extend(helpers::gen_space());
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            "else" => {
                items.extend(helpers::gen_space());
                items.push_string("else".to_string());
                items.extend(helpers::gen_space());
            }
            "if_statement" => {
                // else if: recursively format
                items.extend(gen_if_statement(child, context));
            }
            _ if child.is_named() => {
                // Non-block consequence (single statement) - wrap in block style
                items.extend(gen_node(child, context));
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
    items.push_string("for".to_string());
    items.extend(helpers::gen_space());
    items.push_string("(".to_string());

    // Use field-based access for cleaner for-statement formatting
    if let Some(init) = node.child_by_field_name("init") {
        items.extend(gen_node(init, context));
    }
    // The init (local_variable_declaration) includes its own ";"
    // but we need a space after it
    items.extend(helpers::gen_space());

    if let Some(condition) = node.child_by_field_name("condition") {
        items.extend(gen_node(condition, context));
    }
    items.push_string(";".to_string());
    items.extend(helpers::gen_space());

    if let Some(update) = node.child_by_field_name("update") {
        items.extend(gen_node(update, context));
    }
    items.push_string(")".to_string());

    if let Some(body) = node.child_by_field_name("body") {
        items.extend(helpers::gen_space());
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
    items.push_string("for".to_string());
    items.extend(helpers::gen_space());
    items.push_string("(".to_string());

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
                need_space = true;
            }
            ":" => {
                items.extend(helpers::gen_space());
                items.push_string(":".to_string());
                items.extend(helpers::gen_space());
                need_space = false;
            }
            "block" => {
                items.push_string(")".to_string());
                items.extend(helpers::gen_space());
                items.extend(gen_block(child, context));
                return items;
            }
            _ if child.is_named() => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_node(child, context));
                need_space = true;
            }
            _ => {}
        }
    }

    items.push_string(")".to_string());
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
                items.push_string("while".to_string());
                items.extend(helpers::gen_space());
            }
            "parenthesized_expression" | "condition" => {
                items.extend(gen_node(child, context));
                items.extend(helpers::gen_space());
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
                items.push_string("do".to_string());
                items.extend(helpers::gen_space());
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            "while" => {
                items.extend(helpers::gen_space());
                items.push_string("while".to_string());
                items.extend(helpers::gen_space());
            }
            "parenthesized_expression" => {
                items.extend(gen_node(child, context));
            }
            ";" => {
                items.push_string(";".to_string());
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
                items.push_string("switch".to_string());
                items.extend(helpers::gen_space());
            }
            "parenthesized_expression" => {
                items.extend(gen_node(child, context));
                items.extend(helpers::gen_space());
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
    items.push_string("{".to_string());

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    let cases: Vec<_> = children
        .iter()
        .filter(|c| c.is_named())
        .collect();

    if cases.is_empty() {
        items.push_string("}".to_string());
        return items;
    }

    items.push_signal(Signal::StartIndent);

    for case in &cases {
        items.push_signal(Signal::NewLine);
        items.extend(gen_switch_case(**case, context));
    }

    items.push_signal(Signal::FinishIndent);
    items.push_signal(Signal::NewLine);
    items.push_string("}".to_string());

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
            let body_stmts: Vec<_> = children.iter()
                .skip_while(|c| c.kind() != ":")
                .skip(1) // skip the colon itself
                .filter(|c| c.is_named())
                .collect();

            // Check if the body is a single block
            let is_single_block = body_stmts.len() == 1 && body_stmts[0].kind() == "block";

            for child in &children {
                if child.kind() == "switch_label" {
                    if label_done {
                        items.push_signal(Signal::NewLine);
                    }
                    items.extend(gen_switch_label(*child, context));
                    label_done = true;
                } else if child.kind() == ":" {
                    // Colon is a child of switch_block_statement_group, not switch_label
                    items.push_string(":".to_string());
                    // If the body is a single block, add a space (brace goes on same line)
                    if is_single_block {
                        items.extend(helpers::gen_space());
                    }
                } else if child.is_named() {
                    if !is_single_block {
                        // Multiple statements or non-block: indent and place on new lines
                        if !in_body {
                            items.push_signal(Signal::StartIndent);
                            in_body = true;
                        }
                        items.push_signal(Signal::NewLine);
                    }
                    items.extend(gen_node(*child, context));
                }
            }
            if in_body {
                items.push_signal(Signal::FinishIndent);
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
                        items.extend(helpers::gen_space());
                        items.push_string("->".to_string());
                        items.extend(helpers::gen_space());
                    }
                    _ if child.is_named() => {
                        items.extend(gen_node(*child, context));
                    }
                    _ => {}
                }
            }
        }
        _ => {
            items.extend(helpers::gen_node_text(node, context.source));
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
                items.push_string("case".to_string());
                items.extend(helpers::gen_space());
            }
            "default" => {
                items.push_string("default".to_string());
            }
            ":" => {
                items.push_string(":".to_string());
            }
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
                items.push_string("try".to_string());
                items.extend(helpers::gen_space());
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            "catch_clause" => {
                items.extend(helpers::gen_space());
                items.extend(gen_catch_clause(child, context));
            }
            "finally_clause" => {
                items.extend(helpers::gen_space());
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
                items.push_string("try".to_string());
                items.extend(helpers::gen_space());
            }
            "resource_specification" => {
                items.extend(gen_resource_specification(child, context));
                items.extend(helpers::gen_space());
            }
            "block" => {
                items.extend(gen_block(child, context));
            }
            "catch_clause" => {
                items.extend(helpers::gen_space());
                items.extend(gen_catch_clause(child, context));
            }
            "finally_clause" => {
                items.extend(helpers::gen_space());
                items.extend(gen_finally_clause(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a catch clause: `catch (Exception e) { }`
fn gen_catch_clause<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "catch" => {
                items.push_string("catch".to_string());
                items.extend(helpers::gen_space());
            }
            "catch_formal_parameter" => {
                items.push_string("(".to_string());
                items.extend(gen_catch_formal_parameter(child, context));
                items.push_string(")".to_string());
                items.extend(helpers::gen_space());
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
                    items.extend(helpers::gen_space());
                }
                items.extend(gen_catch_type(child, context));
                need_space = true;
            }
            "identifier" => {
                if need_space {
                    items.extend(helpers::gen_space());
                }
                items.extend(helpers::gen_node_text(child, context.source));
            }
            _ => {}
        }
    }

    items
}

/// Format a catch type: `Exception | RuntimeException`
fn gen_catch_type<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "|" => {
                items.extend(helpers::gen_space());
                items.push_string("|".to_string());
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
                items.push_string("finally".to_string());
                items.extend(helpers::gen_space());
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

    items.push_string("(".to_string());

    for child in node.children(&mut cursor) {
        match child.kind() {
            "(" | ")" => {}
            ";" => {
                items.push_string(";".to_string());
                items.extend(helpers::gen_space());
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

    items.push_string(")".to_string());
    items
}

/// Format a return statement: `return expr;`
pub fn gen_return_statement<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    items.push_string("return".to_string());

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "return" => {}
            ";" => items.push_string(";".to_string()),
            _ if child.is_named() => {
                items.extend(helpers::gen_space());
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
    items.push_string("throw".to_string());

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "throw" => {}
            ";" => items.push_string(";".to_string()),
            _ if child.is_named() => {
                items.extend(helpers::gen_space());
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
    items.push_string("break".to_string());

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "break" => {}
            ";" => items.push_string(";".to_string()),
            "identifier" => {
                items.extend(helpers::gen_space());
                items.extend(helpers::gen_node_text(child, context.source));
            }
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
    items.push_string("continue".to_string());

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "continue" => {}
            ";" => items.push_string(";".to_string()),
            "identifier" => {
                items.extend(helpers::gen_space());
                items.extend(helpers::gen_node_text(child, context.source));
            }
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
    items.push_string("yield".to_string());

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "yield" => {}
            ";" => items.push_string(";".to_string()),
            _ if child.is_named() => {
                items.extend(helpers::gen_space());
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
                items.push_string("synchronized".to_string());
                items.extend(helpers::gen_space());
            }
            "parenthesized_expression" => {
                items.extend(gen_node(child, context));
                items.extend(helpers::gen_space());
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
    items.push_string("assert".to_string());

    let mut cursor = node.walk();
    let mut after_colon = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "assert" => {}
            ":" => {
                items.extend(helpers::gen_space());
                items.push_string(":".to_string());
                after_colon = true;
            }
            ";" => items.push_string(";".to_string()),
            _ if child.is_named() => {
                items.extend(helpers::gen_space());
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
                items.extend(helpers::gen_node_text(child, context.source));
            }
            ":" => {
                items.push_string(":".to_string());
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
