use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use super::context::FormattingContext;
use super::declarations;
use super::generate::gen_node;
use super::helpers;

/// Collapse whitespace in a string: replace newlines and multiple spaces with single spaces.
/// This helps estimate the "flat" width of a code fragment as if formatted on one line.
pub(super) fn collapse_whitespace(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }
    result.trim().to_string()
}

/// Check if a binary expression's `+` operator is being used for string concatenation.
/// Returns true if at least one operand is a string_literal or is itself a string concatenation.
fn is_string_concat(node: tree_sitter::Node, source: &str) -> bool {
    if node.kind() != "binary_expression" {
        return false;
    }
    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();
    let op = children
        .iter()
        .find(|c| !c.is_named())
        .map(|c| &source[c.start_byte()..c.end_byte()]);
    if op != Some("+") {
        return false;
    }
    children.iter().filter(|c| c.is_named()).any(|c| {
        c.kind() == "string_literal"
            || (c.kind() == "binary_expression" && is_string_concat(*c, source))
    })
}

/// Check if a binary expression operator is one we should consider for wrapping.
/// This includes logical operators (&&, ||) and string concatenation (+).
fn is_wrappable_op(op: Option<&str>, node: tree_sitter::Node, source: &str) -> bool {
    match op {
        Some("&&") | Some("||") => true,
        Some("+") => is_string_concat(node, source),
        _ => false,
    }
}

/// Find the start byte of the containing statement/expression for line width calculation.
/// Walks up through parent nodes to find the outermost construct that starts the logical line.
fn find_line_start_byte(node: tree_sitter::Node) -> usize {
    let mut current = node;
    loop {
        if let Some(parent) = current.parent() {
            match parent.kind() {
                "variable_declarator" => {
                    if let Some(grandparent) = parent.parent() {
                        match grandparent.kind() {
                            "local_variable_declaration" | "field_declaration" => {
                                return grandparent.start_byte();
                            }
                            _ => return current.start_byte(),
                        }
                    }
                    return current.start_byte();
                }
                "parenthesized_expression" => {
                    current = parent;
                    continue;
                }
                "if_statement" | "while_statement" | "do_statement" => {
                    return parent.start_byte();
                }
                "return_statement" | "throw_statement" => {
                    return parent.start_byte();
                }
                "argument_list" => {
                    if let Some(grandparent) = parent.parent() {
                        match grandparent.kind() {
                            "method_invocation"
                            | "object_creation_expression"
                            | "explicit_constructor_invocation" => {
                                return grandparent.start_byte();
                            }
                            _ => {}
                        }
                    }
                    return current.start_byte();
                }
                _ => return current.start_byte(),
            }
        } else {
            return current.start_byte();
        }
    }
}

/// Format a binary expression: `a + b`, `x && y`, etc.
///
/// For long chains of `&&`, `||`, or string `+` operators, wraps before each
/// operator with 8-space continuation indent (PJF style):
/// ```java
/// return Utils.enhancedDeepEquals(this.contentType, other.contentType)
///         && Utils.enhancedDeepEquals(this.statusCode, other.statusCode)
///         && Utils.enhancedDeepEquals(this.rawResponse, other.rawResponse);
/// ```
///
/// Also wraps long string concatenation:
/// ```java
/// throw new IllegalStateException("First part of message. "
///         + "Second part of message.");
/// ```
pub fn gen_binary_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut cursor = node.walk();
    let operator = node
        .children(&mut cursor)
        .find(|c| !c.is_named())
        .map(|c| context.source[c.start_byte()..c.end_byte()].to_string());

    let is_wrappable = is_wrappable_op(operator.as_deref(), node, context.source);

    if is_wrappable {
        let is_nested_in_chain = if let Some(parent) = node.parent() {
            if parent.kind() == "binary_expression" {
                let parent_children: Vec<_> = parent.children(&mut parent.walk()).collect();
                let right_child = parent_children.iter().rev().find(|c| c.is_named());
                if let Some(right) = right_child {
                    if right.id() == node.id() {
                        let parent_op = parent_children
                            .iter()
                            .find(|c| !c.is_named())
                            .map(|c| context.source[c.start_byte()..c.end_byte()].to_string());
                        is_wrappable_op(parent_op.as_deref(), parent, context.source)
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !is_nested_in_chain {
            let (operands, operators) = flatten_wrappable_chain(node, context.source);

            let should_wrap = {
                let indent_width = context.indent_level() * context.config.indent_width as usize;

                let line_start_byte = find_line_start_byte(node);

                // Find the end of the source line to include trailing content like `) {` or `;`
                let line_end_byte = context.source[node.end_byte()..]
                    .find('\n')
                    .map(|pos| node.end_byte() + pos)
                    .unwrap_or(context.source.len());

                let line_text = &context.source[line_start_byte..line_end_byte];
                let line_flat_width: usize =
                    line_text.lines().map(|l| l.trim().len()).sum::<usize>()
                        + line_text.lines().count().saturating_sub(1);

                indent_width + line_flat_width > context.config.line_width as usize
            };

            if should_wrap {
                let mut items = PrintItems::new();

                items.extend(gen_node(operands[0], context));
                items.push_signal(Signal::StartIndent);
                items.push_signal(Signal::StartIndent);

                for (i, op) in operators.iter().enumerate() {
                    items.push_signal(Signal::NewLine);
                    items.push_string(op.to_string());
                    items.extend(helpers::gen_space());
                    items.extend(gen_node(operands[i + 1], context));
                }

                items.push_signal(Signal::FinishIndent);
                items.push_signal(Signal::FinishIndent);

                return items;
            }
        }
    }

    // Default: inline formatting
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.is_named() {
            items.extend(gen_node(child, context));
        } else {
            let op = &context.source[child.start_byte()..child.end_byte()];
            items.extend(helpers::gen_space());
            items.push_string(op.to_string());
            items.extend(helpers::gen_space());
        }
    }

    items
}

/// Flatten a chain of binary expressions with wrappable operators (&&, ||, string +).
/// Returns (operands, operators) where operands[i] op operators[i] = operands[i+1].
fn flatten_wrappable_chain<'a>(
    node: tree_sitter::Node<'a>,
    source: &str,
) -> (Vec<tree_sitter::Node<'a>>, Vec<String>) {
    let mut operands = Vec::new();
    let mut operators = Vec::new();

    fn collect<'a>(
        node: tree_sitter::Node<'a>,
        source: &str,
        operands: &mut Vec<tree_sitter::Node<'a>>,
        operators: &mut Vec<String>,
    ) {
        if node.kind() != "binary_expression" {
            operands.push(node);
            return;
        }

        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        let op = children
            .iter()
            .find(|c| !c.is_named())
            .map(|c| source[c.start_byte()..c.end_byte()].to_string());

        let op_str = op.as_deref();
        let is_wrappable = match op_str {
            Some("&&") | Some("||") => true,
            Some("+") => is_string_concat(node, source),
            _ => false,
        };
        if !is_wrappable {
            operands.push(node);
            return;
        }

        let left = children.iter().find(|c| c.is_named()).unwrap();
        let right = children.iter().rev().find(|c| c.is_named()).unwrap();

        collect(*left, source, operands, operators);
        operators.push(op.unwrap());
        collect(*right, source, operands, operators);
    }

    collect(node, source, &mut operands, &mut operators);
    (operands, operators)
}

/// Format a unary expression: `!x`, `-y`, `~z`
pub fn gen_unary_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.is_named() {
            items.extend(gen_node(child, context));
        } else {
            items.extend(helpers::gen_node_text(child, context.source));
        }
    }

    items
}

/// Format an update expression: `i++`, `--j`
pub fn gen_update_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.is_named() {
            items.extend(gen_node(child, context));
        } else {
            items.extend(helpers::gen_node_text(child, context.source));
        }
    }

    items
}

/// Format a method invocation: `obj.method(args)` or `method(args)`
///
/// For chains of 2+ method calls (e.g., `a.b().c().d()`), this flattens the
/// chain and uses PJF-style column-position wrapping: if the column where the
/// first `.` would appear exceeds `method_chain_threshold` (default 80), ALL
/// segments wrap onto new lines with 8-space continuation indent.
pub fn gen_method_invocation<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let depth = chain_depth(node);
    if depth < 1 {
        return gen_method_invocation_simple(node, context);
    }

    // Flatten the chain into (root, [(method_invocation_node, method_name_node, type_args, arg_list, trailing_comment), ...])
    #[allow(clippy::type_complexity)]
    let mut segments: Vec<(
        tree_sitter::Node<'a>,
        tree_sitter::Node<'a>,
        Option<tree_sitter::Node<'a>>,
        Option<tree_sitter::Node<'a>>,
        Option<tree_sitter::Node<'a>>,
    )> = Vec::new();
    let root = flatten_chain(node, &mut segments);

    // Force wrapping if any segment has a lambda with a block body
    let force_wrap = chain_has_lambda_block(&segments);

    // PJF-style chain wrapping: if the chain's flat width (root + all segments) exceeds
    // method_chain_threshold (default 80), wrap ALL segments onto new lines with +8
    // continuation indent. The key PJF difference: ALL segments wrap (including first),
    // rather than keeping the first segment inline with root.
    let root_text = &context.source[root.start_byte()..root.end_byte()];
    let root_width = collapse_whitespace(root_text).len();

    let indent_col = context.indent_level() * (context.config.indent_width as usize);

    // Sum up each segment: . + name + type_args + arg_list + comment
    let mut segments_width = 0;
    for (_, name_node, type_args, arg_list, trailing_comment) in &segments {
        segments_width += 1; // for the '.'
        let name_text = &context.source[name_node.start_byte()..name_node.end_byte()];
        segments_width += name_text.len();

        if let Some(ta) = type_args {
            let ta_text = &context.source[ta.start_byte()..ta.end_byte()];
            segments_width += collapse_whitespace(ta_text).len();
        }

        if let Some(al) = arg_list {
            let al_text = &context.source[al.start_byte()..al.end_byte()];
            segments_width += collapse_whitespace(al_text).len();
        }

        if let Some(tc) = trailing_comment {
            let tc_text = &context.source[tc.start_byte()..tc.end_byte()];
            segments_width += 1 + tc_text.len(); // space + comment
        }
    }

    let chain_flat_width = root_width + segments_width;

    // Use method_chain_threshold for the flat-width decision.
    // For chains in argument lists, use a more aggressive threshold to account for
    // the extra indentation from the containing argument list.
    let is_in_argument_list = context.has_ancestor("argument_list");
    let threshold = if is_in_argument_list {
        (context.config.line_width as usize) / 2
    } else {
        context.config.method_chain_threshold as usize
    };

    // Also wrap if the full line (indent + chain) exceeds line_width
    let line_width = context.config.line_width as usize;

    let should_wrap =
        force_wrap || chain_flat_width > threshold || (indent_col + chain_flat_width) > line_width;

    let mut items = PrintItems::new();
    items.extend(gen_node(root, context));

    if should_wrap {
        // PJF-style wrapping with column-position check:
        // Compute the width of root + first segment to decide if the first segment
        // stays inline or wraps too.
        let first_seg_width =
            if let Some((_, name_node, type_args, arg_list, trailing_comment)) = segments.first() {
                let mut w = 1; // '.'
                let name_text = &context.source[name_node.start_byte()..name_node.end_byte()];
                w += name_text.len();
                if let Some(ta) = type_args {
                    let ta_text = &context.source[ta.start_byte()..ta.end_byte()];
                    w += collapse_whitespace(ta_text).len();
                }
                if let Some(al) = arg_list {
                    let al_text = &context.source[al.start_byte()..al.end_byte()];
                    w += collapse_whitespace(al_text).len();
                }
                if let Some(tc) = trailing_comment {
                    let tc_text = &context.source[tc.start_byte()..tc.end_byte()];
                    w += 1 + tc_text.len();
                }
                w
            } else {
                0
            };

        // If root + first segment exceeds the column threshold, wrap ALL segments
        // (including first). Otherwise, keep first segment inline with root.
        let wrap_first = (indent_col + root_width + first_seg_width) > threshold;

        if wrap_first {
            // ALL segments wrap (including first)
            items.push_signal(Signal::StartIndent);
            items.push_signal(Signal::StartIndent);
            // Track continuation indent for argument list width calculations
            context.add_continuation_indent(2);
            let mut prev_had_comment = false;
            for (_, name_node, type_args, arg_list, trailing_comment) in &segments {
                if !prev_had_comment {
                    items.push_signal(Signal::NewLine);
                }
                items.push_string(".".to_string());
                if let Some(ta) = type_args {
                    items.extend(gen_node(*ta, context));
                }
                items.extend(helpers::gen_node_text(*name_node, context.source));
                if let Some(al) = arg_list {
                    items.extend(gen_node(*al, context));
                }
                if let Some(tc) = trailing_comment {
                    items.extend(helpers::gen_space());
                    items.extend(gen_node(*tc, context));
                    prev_had_comment = true;
                } else {
                    prev_had_comment = false;
                }
            }
            context.remove_continuation_indent(2);
            items.push_signal(Signal::FinishIndent);
            items.push_signal(Signal::FinishIndent);
        } else {
            // Keep first segment inline with root, wrap subsequent segments
            if let Some((_, name_node, type_args, arg_list, trailing_comment)) = segments.first() {
                items.push_string(".".to_string());
                if let Some(ta) = type_args {
                    items.extend(gen_node(*ta, context));
                }
                items.extend(helpers::gen_node_text(*name_node, context.source));
                if let Some(al) = arg_list {
                    items.extend(gen_node(*al, context));
                }
                if let Some(tc) = trailing_comment {
                    items.extend(helpers::gen_space());
                    items.extend(gen_node(*tc, context));
                }
            }

            if segments.len() > 1 {
                items.push_signal(Signal::StartIndent);
                items.push_signal(Signal::StartIndent);
                // Track continuation indent for argument list width calculations
                context.add_continuation_indent(2);
                let mut prev_had_comment =
                    segments.first().and_then(|(_, _, _, _, tc)| *tc).is_some();
                for (_, name_node, type_args, arg_list, trailing_comment) in &segments[1..] {
                    if !prev_had_comment {
                        items.push_signal(Signal::NewLine);
                    }
                    items.push_string(".".to_string());
                    if let Some(ta) = type_args {
                        items.extend(gen_node(*ta, context));
                    }
                    items.extend(helpers::gen_node_text(*name_node, context.source));
                    if let Some(al) = arg_list {
                        items.extend(gen_node(*al, context));
                    }
                    if let Some(tc) = trailing_comment {
                        items.extend(helpers::gen_space());
                        items.extend(gen_node(*tc, context));
                        prev_had_comment = true;
                    } else {
                        prev_had_comment = false;
                    }
                }
                context.remove_continuation_indent(2);
                items.push_signal(Signal::FinishIndent);
                items.push_signal(Signal::FinishIndent);
            }
        }
    } else {
        // Keep on one line
        for (_, name_node, type_args, arg_list, trailing_comment) in segments {
            items.push_string(".".to_string());
            if let Some(ta) = type_args {
                items.extend(gen_node(ta, context));
            }
            items.extend(helpers::gen_node_text(name_node, context.source));
            if let Some(al) = arg_list {
                items.extend(gen_node(al, context));
            }
            // Emit trailing comment if present
            if let Some(tc) = trailing_comment {
                items.extend(helpers::gen_space());
                items.extend(gen_node(tc, context));
            }
        }
    }

    items
}

/// Simple (non-chained) method invocation: `method(args)` or `obj.method(args)`
fn gen_method_invocation_simple<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "." => {
                items.push_string(".".to_string());
            }
            "identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "argument_list" => {
                items.extend(gen_node(child, context));
            }
            "type_arguments" => {
                items.extend(gen_node(child, context));
            }
            "line_comment" if child.is_extra() => {
                // Line comment within the method invocation (e.g., after argument list)
                // Add space before comment, then emit it (which will add newline)
                items.extend(helpers::gen_space());
                items.extend(super::comments::gen_line_comment(child, context));
            }
            "block_comment" if child.is_extra() => {
                // Block comment within the method invocation
                items.extend(helpers::gen_space());
                items.extend(super::comments::gen_block_comment(child, context));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Check if any argument list in a chain segment contains a lambda with a block body.
/// This is used to force chain wrapping when lambdas with block bodies are present,
/// since the multi-line block content would produce incorrect indentation on a single line.
#[allow(clippy::type_complexity)]
fn chain_has_lambda_block(
    segments: &[(
        tree_sitter::Node,
        tree_sitter::Node,
        Option<tree_sitter::Node>,
        Option<tree_sitter::Node>,
        Option<tree_sitter::Node>,
    )],
) -> bool {
    for (_, _, _, arg_list, _) in segments {
        if let Some(al) = arg_list
            && arg_list_has_lambda_block(*al)
        {
            return true;
        }
    }
    false
}

/// Check if an argument list contains a lambda expression with a block body.
fn arg_list_has_lambda_block(node: tree_sitter::Node) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "lambda_expression" {
            let mut lc = child.walk();
            for lchild in child.children(&mut lc) {
                if lchild.kind() == "block" {
                    return true;
                }
            }
        }
    }
    false
}

/// Count how deep a method invocation chain is (number of nested method_invocations).
/// `a.b()` = 0, `a.b().c()` = 1, `a.b().c().d()` = 2, etc.
pub(super) fn chain_depth(node: tree_sitter::Node) -> usize {
    let mut depth = 0;
    let mut current = node;
    loop {
        let mut cursor = current.walk();
        let object = current
            .children(&mut cursor)
            .find(|c| c.is_named() && c.kind() != "argument_list" && c.kind() != "type_arguments");
        match object {
            Some(obj) if obj.kind() == "method_invocation" => {
                depth += 1;
                current = obj;
            }
            _ => break,
        }
    }
    depth
}

/// Flatten a nested method_invocation chain into segments.
/// Returns the root object node (the non-method-invocation at the bottom).
/// Segments are collected in call order (first call first).
/// Each segment is (invocation_node, name_node, type_args, arg_list).
/// Extract trailing line comment that appears on the same line as the given node
fn extract_trailing_line_comment<'a>(node: tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
    let node_end_row = node.end_position().row;

    // Look for a line_comment sibling that starts on the same row
    let mut next = node.next_sibling();
    while let Some(sibling) = next {
        if sibling.kind() == "line_comment" {
            if sibling.start_position().row == node_end_row {
                return Some(sibling);
            }
            return None; // Comment on different line
        }
        if !sibling.is_extra() {
            return None; // Non-comment node in the way
        }
        next = sibling.next_sibling();
    }
    None
}

#[allow(clippy::type_complexity)]
fn flatten_chain<'a>(
    node: tree_sitter::Node<'a>,
    segments: &mut Vec<(
        tree_sitter::Node<'a>,
        tree_sitter::Node<'a>,
        Option<tree_sitter::Node<'a>>,
        Option<tree_sitter::Node<'a>>,
        Option<tree_sitter::Node<'a>>,
    )>,
) -> tree_sitter::Node<'a> {
    // Collect the chain in reverse (innermost first), then reverse at the end.
    let mut chain = Vec::new();
    let mut current = node;

    loop {
        // tree-sitter method_invocation has named fields: "object", "name", "arguments"
        let object = current.child_by_field_name("object");
        let name = current.child_by_field_name("name");
        let type_args = {
            let mut cursor = current.walk();
            current
                .children(&mut cursor)
                .find(|c| c.kind() == "type_arguments")
        };
        let arg_list = current.child_by_field_name("arguments");

        // Check for trailing line comment on this segment
        let trailing_comment = extract_trailing_line_comment(current);

        if let Some(name_node) = name {
            chain.push((current, name_node, type_args, arg_list, trailing_comment));
        }

        match object {
            Some(obj) if obj.kind() == "method_invocation" => {
                current = obj;
            }
            Some(obj) => {
                // Root object (e.g., field_access, identifier, etc.)
                chain.reverse();
                segments.extend(chain);
                return obj;
            }
            None => {
                // No object — bare method call at the root of the chain.
                // Pop the root entry from chain; the caller's gen_node(root)
                // will format the bare call via gen_method_invocation_simple.
                chain.pop();
                chain.reverse();
                segments.extend(chain);
                return current;
            }
        }
    }
}

/// Format a field access: `obj.field`
pub fn gen_field_access<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "." => {
                items.push_string(".".to_string());
            }
            "identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "this" | "super" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a lambda expression: `x -> x + 1` or `(x, y) -> { body }`
pub fn gen_lambda_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "inferred_parameters" => {
                items.extend(gen_node(child, context));
            }
            "formal_parameters" => {
                items.extend(declarations::gen_formal_parameters(child, context));
            }
            "->" => {
                items.extend(helpers::gen_space());
                items.push_string("->".to_string());
                items.extend(helpers::gen_space());
            }
            "block" => {
                items.extend(gen_node(child, context));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format a ternary expression: `cond ? a : b`
///
/// When the full ternary expression would exceed `line_width`, wraps before
/// `?` and `:` with 8-space continuation indent (PJF style):
/// ```java
/// String reason = e instanceof RetryableException
///         ? "status " + ((RetryableException) e).response().statusCode()
///         : e.getClass().getSimpleName();
/// ```
pub fn gen_ternary_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    // Estimate the "flat" width of the entire ternary expression (as if on one line).
    let ternary_text = &context.source[node.start_byte()..node.end_byte()];
    let ternary_flat_width: usize = ternary_text.lines().map(|l| l.trim().len()).sum::<usize>()
        + ternary_text.lines().count().saturating_sub(1); // spaces between joined lines

    let indent_width = context.indent_level() * context.config.indent_width as usize;
    let should_wrap = indent_width + ternary_flat_width > context.config.line_width as usize;

    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    if should_wrap {
        // Wrapped: break before ? and : with 8-space continuation indent
        let mut started_indent = false;
        for child in node.children(&mut cursor) {
            match child.kind() {
                "?" => {
                    if !started_indent {
                        items.push_signal(Signal::StartIndent);
                        items.push_signal(Signal::StartIndent);
                        started_indent = true;
                    }
                    items.push_signal(Signal::NewLine);
                    items.push_string("?".to_string());
                    items.extend(helpers::gen_space());
                }
                ":" => {
                    items.push_signal(Signal::NewLine);
                    items.push_string(":".to_string());
                    items.extend(helpers::gen_space());
                }
                _ if child.is_named() => {
                    items.extend(gen_node(child, context));
                }
                _ => {}
            }
        }
        if started_indent {
            items.push_signal(Signal::FinishIndent);
            items.push_signal(Signal::FinishIndent);
        }
    } else {
        // Inline: keep everything on one line
        for child in node.children(&mut cursor) {
            match child.kind() {
                "?" => {
                    items.extend(helpers::gen_space());
                    items.push_string("?".to_string());
                    items.extend(helpers::gen_space());
                }
                ":" => {
                    items.extend(helpers::gen_space());
                    items.push_string(":".to_string());
                    items.extend(helpers::gen_space());
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

/// Format an object creation expression: `new Foo(args)`, `new Foo() { ... }`
pub fn gen_object_creation_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "new" => {
                items.push_string("new".to_string());
                items.extend(helpers::gen_space());
            }
            "type_arguments" => {
                items.extend(gen_node(child, context));
            }
            "type_identifier" | "scoped_type_identifier" | "generic_type" => {
                items.extend(gen_node(child, context));
            }
            "argument_list" => {
                items.extend(gen_node(child, context));
            }
            "class_body" => {
                items.extend(helpers::gen_space());
                items.extend(gen_node(child, context));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format an array creation expression: `new int[n]`, `new int[]{1, 2, 3}`
pub fn gen_array_creation_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "new" => {
                items.push_string("new".to_string());
                items.extend(helpers::gen_space());
            }
            "dimensions_expr" => {
                items.extend(gen_node(child, context));
            }
            "dimensions" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            "array_initializer" => {
                items.extend(gen_array_initializer(child, context));
            }
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format an array initializer: `{1, 2, 3}`
///
/// When the initializer contains comments (is_extra() children), expands to
/// one element per line to match PJF behavior.
///
/// When the parent is an annotation context (element_value_pair or
/// annotation_argument_list) and there are multiple elements, forces
/// one-element-per-line format with trailing comma, matching PJF behavior.
pub fn gen_array_initializer<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    // Check if this array initializer has any comments
    let has_comments = node.children(&mut cursor).any(|c| c.is_extra());

    // Check if this is inside an annotation context
    let in_annotation = node
        .parent()
        .map(|p| {
            p.kind() == "annotation_argument_list"
                || p.kind() == "element_value_pair"
                || (p.kind() == "annotation_argument_list"
                    && p.parent()
                        .is_some_and(|gp| gp.kind().contains("annotation")))
        })
        .unwrap_or(false);

    // Count named (element) children
    cursor = node.walk();
    let element_count = node.children(&mut cursor).filter(|c| c.is_named()).count();

    // Force expanded format in annotation context with multiple elements,
    // but only if the annotation wouldn't fit on one line
    let force_expand = if in_annotation && element_count > 1 {
        // Find the annotation node to check the full width
        let mut current = node;
        let mut should_expand = true; // Default to expanding if annotation not found
        while let Some(parent) = current.parent() {
            if parent.kind() == "marker_annotation"
                || parent.kind() == "annotation"
                || parent.kind() == "normal_annotation"
            {
                // Compute flat width of the entire annotation
                let ann_text = &context.source[parent.start_byte()..parent.end_byte()];
                let flat_width = collapse_whitespace(ann_text).len();
                let indent_col =
                    context.effective_indent_level() * context.config.indent_width as usize;
                should_expand = indent_col + flat_width > context.config.line_width as usize;
                break;
            }
            current = parent;
        }
        should_expand
    } else {
        false
    };

    // Reset cursor for iteration
    cursor = node.walk();

    items.push_string("{".to_string());

    if has_comments || force_expand {
        // Expanded format: one element per line
        items.push_signal(Signal::StartIndent);
        let mut prev_was_line_comment = false;

        // Collect named children so we can add trailing comma
        let all_children: Vec<_> = node.children(&mut cursor).collect();

        // Count total named children for trailing comma logic
        let named_count = all_children.iter().filter(|c| c.is_named()).count();
        let mut named_idx = 0;

        for child in &all_children {
            match child.kind() {
                "{" | "}" => {}
                "," => {
                    items.push_string(",".to_string());
                }
                _ if child.is_extra() => {
                    // Comment node
                    if !prev_was_line_comment {
                        items.push_signal(Signal::NewLine);
                    }
                    items.extend(gen_node(*child, context));
                    prev_was_line_comment = child.kind() == "line_comment";
                }
                _ if child.is_named() => {
                    // Element node
                    if !prev_was_line_comment {
                        items.push_signal(Signal::NewLine);
                    }
                    items.extend(gen_node(*child, context));
                    named_idx += 1;

                    // Add trailing comma after last element in annotation context
                    // (PJF always adds trailing comma in expanded arrays)
                    if force_expand && named_idx == named_count {
                        // Check if there's already a comma following this element
                        let has_trailing_comma = all_children
                            .iter()
                            .skip_while(|c| c.id() != child.id())
                            .skip(1)
                            .any(|c| c.kind() == ",");
                        if !has_trailing_comma {
                            items.push_string(",".to_string());
                        }
                    }

                    prev_was_line_comment = false;
                }
                _ => {}
            }
        }

        if !prev_was_line_comment {
            items.push_signal(Signal::NewLine);
        }
        items.push_signal(Signal::FinishIndent);
    } else {
        // Compact format: inline
        let mut first = true;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "{" | "}" => {}
                "," => {
                    items.push_string(",".to_string());
                    items.extend(helpers::gen_space());
                }
                _ if child.is_named() => {
                    if first {
                        // No leading space for compact initializers
                    }
                    items.extend(gen_node(child, context));
                    first = false;
                }
                _ => {}
            }
        }
    }

    items.push_string("}".to_string());
    items
}

/// Format an array access: `arr[i]`
pub fn gen_array_access<'a>(
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

/// Format a cast expression: `(Type) expr`
pub fn gen_cast_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();
    let mut after_type = false;

    for child in node.children(&mut cursor) {
        match child.kind() {
            "(" => items.push_string("(".to_string()),
            ")" => {
                items.push_string(")".to_string());
                items.extend(helpers::gen_space());
                after_type = true;
            }
            _ if child.is_named() && !after_type => {
                // The type inside the cast
                items.extend(gen_node(child, context));
            }
            _ if child.is_named() && after_type => {
                // The expression being cast
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
}

/// Format an instanceof expression: `expr instanceof Type`
pub fn gen_instanceof_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "instanceof" => {
                items.extend(helpers::gen_space());
                items.push_string("instanceof".to_string());
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

/// Format a parenthesized expression: `(expr)`
pub fn gen_parenthesized_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "(" => items.push_string("(".to_string()),
            ")" => items.push_string(")".to_string()),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}

/// Format a method reference: `Class::method`, `obj::method`
pub fn gen_method_reference<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "::" => items.push_string("::".to_string()),
            "new" => items.push_string("new".to_string()),
            "identifier" => items.extend(helpers::gen_node_text(child, context.source)),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}

/// Format an assignment expression: `x = y`, `x += y`
pub fn gen_assignment_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.is_named() {
            items.extend(gen_node(child, context));
        } else {
            let op = &context.source[child.start_byte()..child.end_byte()];
            items.extend(helpers::gen_space());
            items.push_string(op.to_string());
            items.extend(helpers::gen_space());
        }
    }

    items
}

/// Format an inferred parameters list: `(x, y)` in lambdas
pub fn gen_inferred_parameters<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "(" => items.push_string("(".to_string()),
            ")" => items.push_string(")".to_string()),
            "," => {
                items.push_string(",".to_string());
                items.extend(helpers::gen_space());
            }
            "identifier" => {
                items.extend(helpers::gen_node_text(child, context.source));
            }
            _ => {}
        }
    }

    items
}

/// Format an explicit constructor invocation: `this(args)` or `super(args)`
pub fn gen_explicit_constructor_invocation<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "this" => items.push_string("this".to_string()),
            "super" => items.push_string("super".to_string()),
            "argument_list" => items.extend(gen_node(child, context)),
            ";" => items.push_string(";".to_string()),
            "type_arguments" => items.extend(gen_node(child, context)),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}
