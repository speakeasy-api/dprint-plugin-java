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
                let start_col = node.start_position().column;
                let expr_text = &context.source[node.start_byte()..node.end_byte()];
                let expr_flat_width: usize =
                    expr_text.lines().map(|l| l.trim().len()).sum::<usize>()
                        + expr_text.lines().count().saturating_sub(1);

                // For conditions inside if/while/for, account for trailing `) {`
                let is_condition = node
                    .parent()
                    .and_then(|p| {
                        if p.kind() == "parenthesized_expression" {
                            p.parent()
                        } else {
                            None
                        }
                    })
                    .map(|gp| {
                        matches!(
                            gp.kind(),
                            "if_statement" | "while_statement" | "for_statement"
                        )
                    })
                    .unwrap_or(false);

                let suffix_width = if is_condition { 3 } else { 0 }; // `) {`

                start_col + expr_flat_width + suffix_width > context.config.line_width as usize
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

    // PJF-style chain wrapping: compute chain "prefix width" — the width of the chain
    // up to (but excluding) lambda block bodies. PJF measures where the chain DOTs fall,
    // not the total content including multi-line lambda bodies.
    let root_text = &context.source[root.start_byte()..root.end_byte()];
    let root_width = collapse_whitespace(root_text).len();

    // When the assignment/variable_declarator has already wrapped at '=',
    // the chain starts at continuation indent with NO prefix on the same line.
    // Adjust indent_col and prefix_width accordingly.
    let indent_width = context.config.indent_width as usize;
    let (indent_col, prefix_width) = if context.is_assignment_wrapped() {
        // Assignment wrapped: chain is at continuation indent, already tracked
        // in effective_indent_level via add_continuation_indent(2)
        let cont_col = context.effective_indent_level() * indent_width;
        (cont_col, 0)
    } else {
        // Use effective_indent_level to include continuation indent from
        // outer chain wrapping and argument list wrapping.
        let col = context.effective_indent_level() * indent_width;
        let prefix = compute_chain_prefix_width(node, context);
        (col, prefix)
    };

    // Sum up each segment: . + name + type_args + arg_list (with lambda body excluded)
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
            // If the argument list contains a lambda with a block body, only count
            // the "header" width up to the opening '{', not the full body content.
            // This matches PJF which measures chain prefix position, not total content.
            segments_width += estimate_arg_list_width(*al, context.source);
        }

        if let Some(tc) = trailing_comment {
            let tc_text = &context.source[tc.start_byte()..tc.end_byte()];
            segments_width += 1 + tc_text.len(); // space + comment
        }
    }

    let chain_flat_width = root_width + segments_width;

    // PJF's METHOD_CHAIN_COLUMN_LIMIT: check if ANY dot's column position exceeds 80.
    // Walk through segments accumulating position. If any dot exceeds the threshold, wrap.
    // Exception: single-invocation chains (root + 1 method) use line_width as threshold
    // per PJF's LastLevelBreakability.ACCEPT_INLINE_CHAIN_IF_SIMPLE optimization.
    let line_width = context.config.line_width as usize;
    let chain_threshold = context.config.method_chain_threshold as usize;
    let effective_chain_threshold = if segments.len() == 1 {
        line_width // Single-method chains only wrap at line_width (120)
    } else {
        chain_threshold // Multi-method chains wrap at column 80
    };

    let mut any_dot_exceeds = false;
    let mut first_exceeding_segment: Option<usize> = None;
    let mut cumulative = root_width;
    for (i, (_, name_node, type_args, arg_list, trailing_comment)) in segments.iter().enumerate() {
        // The dot for this segment appears at cumulative position
        let dot_position = indent_col + prefix_width + cumulative;
        if dot_position > effective_chain_threshold {
            any_dot_exceeds = true;
            if first_exceeding_segment.is_none() {
                first_exceeding_segment = Some(i);
            }
        }
        // Add this segment's width to cumulative
        cumulative += 1; // '.'
        let name_text = &context.source[name_node.start_byte()..name_node.end_byte()];
        cumulative += name_text.len();
        if let Some(ta) = type_args {
            let ta_text = &context.source[ta.start_byte()..ta.end_byte()];
            cumulative += collapse_whitespace(ta_text).len();
        }
        if let Some(al) = arg_list {
            cumulative += estimate_arg_list_width(*al, context.source);
        }
        if let Some(tc) = trailing_comment {
            let tc_text = &context.source[tc.start_byte()..tc.end_byte()];
            cumulative += 1 + tc_text.len();
        }
    }

    // Also check total line width (indent + prefix + chain) against line_width
    // Use >= (not >) to match PJF's strict behavior (line_width is exclusive)
    let effective_position = indent_col + prefix_width + chain_flat_width;
    let should_wrap = any_dot_exceeds || effective_position >= line_width;

    let mut items = PrintItems::new();
    items.extend(gen_node(root, context));

    if should_wrap {
        // PJF chain prefix detection:
        // Determine how many initial segments form the "prefix" (stay inline with root).
        //
        // Two rules (derived from PJF source analysis):
        // 1. If any dot exceeds METHOD_CHAIN_COLUMN_LIMIT (80): everything before that
        //    dot stays inline as prefix, everything from that dot wraps.
        // 2. If no dot exceeds 80 but total exceeds line_width: use zero-arg prefix
        //    (consecutive zero-arg methods from start stay inline).
        // 3. Class-ref roots: always at least 1 prefix (root + first method).
        let root_is_class_ref = {
            let root_text = &context.source[root.start_byte()..root.end_byte()];
            let last_component = root_text.rsplit('.').next().unwrap_or(root_text);
            last_component
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
        };

        // Helper: check if a segment is zero-arg (no args or empty parens)
        let is_seg_zero_arg = |seg: &(
            tree_sitter::Node,
            tree_sitter::Node,
            Option<tree_sitter::Node>,
            Option<tree_sitter::Node>,
            Option<tree_sitter::Node>,
        )|
         -> bool {
            match seg.3 {
                None => true,
                Some(al) => {
                    let al_text = &context.source[al.start_byte()..al.end_byte()];
                    al_text.trim() == "()"
                }
            }
        };

        // PJF prefix rules (verified by testing against PJF 2.50):
        // 1. Class-ref roots: always prefix = 1 (e.g., SDK.builder())
        // 2. Method invocation roots: prefix = 0 (root IS the first call)
        // 3. Identifier/field_access/new expression roots:
        //    PJF uses root text length <= 8 as threshold (matches continuation indent).
        //    Short roots (e.g., sdk, obj, client) keep first segment inline;
        //    long roots (e.g., contextRunner, sdkConfiguration) wrap from root.
        // 4. Stream/parallelStream extends prefix beyond initial count
        let root_text_len = root.end_byte() - root.start_byte();

        let mut prefix_count = if root_is_class_ref {
            1
        } else if root.kind() == "method_invocation" {
            0
        } else if root_text_len <= 8 {
            // Short root → keep first segment inline with root
            1
        } else {
            // Long root → wrap from root
            0
        };

        // PJF extends the prefix to include `.stream()` and `.parallelStream()`
        // methods, plus any zero-arg predecessors leading to them.
        while prefix_count < segments.len() {
            let seg = &segments[prefix_count];
            if !is_seg_zero_arg(seg) {
                break;
            }
            let name = &context.source[seg.1.start_byte()..seg.1.end_byte()];
            if name == "stream" || name == "parallelStream" {
                prefix_count += 1;
                break;
            }
            // Check if a later zero-arg segment is stream/parallelStream
            let has_stream_ahead = segments[prefix_count + 1..].iter().any(|s| {
                if !is_seg_zero_arg(s) {
                    return false;
                }
                let n = &context.source[s.1.start_byte()..s.1.end_byte()];
                n == "stream" || n == "parallelStream"
            });
            if has_stream_ahead {
                prefix_count += 1;
            } else {
                break;
            }
        }

        // Emit prefix segments inline, then wrap the rest
        for (i, (_, name_node, type_args, arg_list, trailing_comment)) in
            segments.iter().enumerate()
        {
            if i < prefix_count {
                // Inline with root (prefix)
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
            } else if i == prefix_count {
                // First wrapping segment — start indent block
                items.push_signal(Signal::StartIndent);
                items.push_signal(Signal::StartIndent);
                context.add_continuation_indent(2);
                // Check if previous prefix segment had a trailing comment
                let prev_had_comment = if i > 0 {
                    segments[i - 1].4.is_some()
                } else {
                    false
                };
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
                }
            } else {
                // Subsequent wrapping segments
                let prev_had_comment = segments[i - 1].4.is_some();
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
                }
            }
        }
        // Close indent block if any segments were wrapped
        if prefix_count < segments.len() {
            context.remove_continuation_indent(2);
            items.push_signal(Signal::FinishIndent);
            items.push_signal(Signal::FinishIndent);
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
/// Estimate argument list width for chain wrapping decisions.
/// If the arg list contains a lambda with a block body, only count the "header"
/// width up to the opening '{', since PJF measures chain prefix position, not
/// total lambda body content.
fn estimate_arg_list_width(arg_list: tree_sitter::Node, source: &str) -> usize {
    // Check if arg list contains a lambda with a block body
    let mut cursor = arg_list.walk();
    let mut has_lambda_block = false;
    for child in arg_list.children(&mut cursor) {
        if child.kind() == "lambda_expression" {
            let mut inner_cursor = child.walk();
            for inner in child.children(&mut inner_cursor) {
                if inner.kind() == "block" {
                    has_lambda_block = true;
                    break;
                }
            }
        }
        if has_lambda_block {
            break;
        }
    }

    if has_lambda_block {
        // Find the opening '{' and count only up to it
        let al_text = &source[arg_list.start_byte()..arg_list.end_byte()];
        if let Some(brace_pos) = al_text.find('{') {
            // Width is from '(' to '{' inclusive
            let header = &al_text[..brace_pos + 1];
            collapse_whitespace(header).len()
        } else {
            collapse_whitespace(al_text).len()
        }
    } else {
        let al_text = &source[arg_list.start_byte()..arg_list.end_byte()];
        collapse_whitespace(al_text).len()
    }
}

/// Check if a method chain would fit inline (without wrapping) at a given column position.
/// Used by gen_variable_declarator to determine if wrapping at '=' allows the chain to stay inline.
#[allow(clippy::type_complexity)]
pub fn chain_fits_inline_at(
    node: tree_sitter::Node,
    col: usize,
    source: &str,
    config: &crate::configuration::Configuration,
) -> bool {
    let mut segments: Vec<(
        tree_sitter::Node,
        tree_sitter::Node,
        Option<tree_sitter::Node>,
        Option<tree_sitter::Node>,
        Option<tree_sitter::Node>,
    )> = Vec::new();
    let root = flatten_chain(node, &mut segments);

    let root_text = &source[root.start_byte()..root.end_byte()];
    let root_width = collapse_whitespace(root_text).len();

    let chain_threshold = config.method_chain_threshold as usize;
    let line_width = config.line_width as usize;

    // Check per-dot positions — if ANY dot exceeds chain threshold, chain needs wrapping
    let mut total_width = root_width;
    for (_, name_node, type_args, arg_list, trailing_comment) in &segments {
        let dot_position = col + total_width;
        if dot_position > chain_threshold {
            return false;
        }
        total_width += 1; // '.'
        let name_text = &source[name_node.start_byte()..name_node.end_byte()];
        total_width += name_text.len();
        if let Some(ta) = type_args {
            let ta_text = &source[ta.start_byte()..ta.end_byte()];
            total_width += collapse_whitespace(ta_text).len();
        }
        if let Some(al) = arg_list {
            total_width += estimate_arg_list_width(*al, source);
        }
        if let Some(tc) = trailing_comment {
            let tc_text = &source[tc.start_byte()..tc.end_byte()];
            total_width += 1 + tc_text.len();
        }
    }

    // Total line position must fit within line_width (strict less-than, matching PJF)
    (col + total_width) < line_width
}

/// Compute the width of content that precedes a chain on the same line.
/// For `this.field = chain.method()`, returns width of "this.field = " (prefix before chain).
/// For `return chain.method()`, returns 7 (for "return ").
/// This lets the chain wrapping decision account for the full line width, not just indent + chain.
fn compute_chain_prefix_width(node: tree_sitter::Node, context: &FormattingContext) -> usize {
    let parent = node.parent();
    match parent.map(|p| p.kind()) {
        Some("assignment_expression") => {
            // e.g., `this.field = chain...` — prefix is LHS + " = "
            if let Some(p) = parent
                && let Some(lhs) = p.child_by_field_name("left")
            {
                let lhs_text = &context.source[lhs.start_byte()..lhs.end_byte()];
                return collapse_whitespace(lhs_text).len() + 3; // " = "
            }
            0
        }
        Some("variable_declarator") => {
            // e.g., `Type var = chain...` — prefix includes type + name + " = "
            // Look at grandparent (local_variable_declaration) for type info
            if let Some(p) = parent
                && let Some(gp) = p.parent()
            {
                let mut type_width = 0;
                let mut cursor = gp.walk();
                for child in gp.children(&mut cursor) {
                    if child.id() == p.id() {
                        break;
                    }
                    if child.is_named() {
                        let text = &context.source[child.start_byte()..child.end_byte()];
                        if type_width > 0 {
                            type_width += 1; // space between tokens
                        }
                        type_width += collapse_whitespace(text).len();
                    }
                }
                // Add variable name width
                if let Some(name) = p.child_by_field_name("name") {
                    let name_text = &context.source[name.start_byte()..name.end_byte()];
                    return type_width + 1 + name_text.len() + 3; // " name = "
                }
            }
            0
        }
        Some("return_statement") => 7, // "return "
        Some("throw_statement") => 6,  // "throw "
        Some("argument_list") => {
            // Chain is an argument in a method/constructor call.
            // If the parent method_invocation is part of a chain, the chain prefix
            // is ".methodName(" which precedes this argument on the same line.
            if let Some(p) = parent
                && let Some(gp) = p.parent()
                && gp.kind() == "method_invocation"
            {
                let in_chain = gp
                    .child_by_field_name("object")
                    .is_some_and(|obj| obj.kind() == "method_invocation")
                    || gp
                        .parent()
                        .is_some_and(|ggp| ggp.kind() == "method_invocation");
                if in_chain && let Some(name) = gp.child_by_field_name("name") {
                    let name_text = &context.source[name.start_byte()..name.end_byte()];
                    return 1 + name_text.len() + 1; // ".name("
                }
            }
            0
        }
        _ => 0,
    }
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

/// Find the rightmost "last dot" position within any method chain in the expression.
/// Returns the column position relative to `base_col` where the last `.method(...)` segment
/// starts. For nested expressions, this walks into arguments to find deeply nested chains.
/// Returns 0 if no chain dots are found.
pub(super) fn rightmost_chain_dot(node: tree_sitter::Node, source: &str, base_col: usize) -> usize {
    let text = &source[node.start_byte()..node.end_byte()];
    let flat_width: usize = text.lines().map(|l| l.trim().len()).sum();

    if node.kind() == "method_invocation" && chain_depth(node) >= 1 {
        // This is a chain. Find the last dot position.
        let name_w = node
            .child_by_field_name("name")
            .map(|n| n.end_byte() - n.start_byte())
            .unwrap_or(0);
        let args_w = node
            .child_by_field_name("arguments")
            .map(|a| {
                let t = &source[a.start_byte()..a.end_byte()];
                t.lines().map(|l| l.trim().len()).sum::<usize>()
            })
            .unwrap_or(0);
        let last_seg_width = 1 + name_w + args_w; // ".name(args)"
        base_col + flat_width.saturating_sub(last_seg_width)
    } else if node.kind() == "method_invocation" {
        // Single method call — check if args contain chains
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            let mut max_dot = 0usize;
            // Compute position of each arg based on preceding text
            for child in args_node.children(&mut cursor) {
                if child.is_named() {
                    let child_offset: usize = {
                        let before = &source[node.start_byte()..child.start_byte()];
                        before.lines().map(|l| l.trim().len()).sum()
                    };
                    let dot_pos = rightmost_chain_dot(child, source, base_col + child_offset);
                    max_dot = max_dot.max(dot_pos);
                }
            }
            max_dot
        } else {
            0
        }
    } else if node.kind() == "binary_expression" {
        // Check both operands of binary expression for chain dots
        let mut cursor = node.walk();
        let mut max_dot = 0usize;
        let mut col = base_col;
        for child in node.children(&mut cursor) {
            if child.is_named() {
                let dot_pos = rightmost_chain_dot(child, source, col);
                max_dot = max_dot.max(dot_pos);
                let child_text = &source[child.start_byte()..child.end_byte()];
                col += child_text.lines().map(|l| l.trim().len()).sum::<usize>();
            } else {
                // Operator like "+", "&&", etc.
                let op_text = &source[child.start_byte()..child.end_byte()];
                col += op_text.len() + 2; // " op "
            }
        }
        max_dot
    } else {
        0
    }
}

/// Compute the width of the chain root + first segment for assignment wrapping decisions.
/// For a chain like `AuthResponse.builder().contentType().statusCode()`, this returns
/// (root_width="AuthResponse", first_seg_width=".builder()") so the caller can check
/// if `LHS = AuthResponse.builder()` fits on one line.
pub fn chain_root_first_seg_width(node: tree_sitter::Node, source: &str) -> (usize, usize) {
    let mut segments = Vec::new();
    let root = flatten_chain(node, &mut segments);

    let root_text = &source[root.start_byte()..root.end_byte()];
    let root_width = collapse_whitespace(root_text).len();

    let first_seg_width = if let Some((_, name_node, type_args, arg_list, _)) = segments.first() {
        let mut w = 1; // '.'
        let name_text = &source[name_node.start_byte()..name_node.end_byte()];
        w += name_text.len();
        if let Some(ta) = type_args {
            let ta_text = &source[ta.start_byte()..ta.end_byte()];
            w += collapse_whitespace(ta_text).len();
        }
        if let Some(al) = arg_list {
            let al_text = &source[al.start_byte()..al.end_byte()];
            w += collapse_whitespace(al_text).len();
        }
        w
    } else {
        0
    };

    (root_width, first_seg_width)
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

/// Format an array creation expression: `new int[n]`, `new int[] {1, 2, 3}`
pub fn gen_array_creation_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    // Check if we have an array_initializer to add space between dimensions and initializer
    let has_initializer = node.child_by_field_name("value").is_some();

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
                // Add space after dimensions if array_initializer follows
                if has_initializer {
                    items.extend(helpers::gen_space());
                }
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

        let all_children: Vec<_> = node.children(&mut cursor).collect();

        for (ci, child) in all_children.iter().enumerate() {
            match child.kind() {
                "{" | "}" => {}
                "," => {
                    // PJF removes trailing commas in annotation arrays but keeps them
                    // in regular Java array initializers.
                    if in_annotation {
                        let has_more_elements = all_children[ci + 1..]
                            .iter()
                            .any(|c| c.is_named() && !c.is_extra());
                        if has_more_elements {
                            items.push_string(",".to_string());
                        }
                    } else {
                        items.push_string(",".to_string());
                    }
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
        let compact_children: Vec<_> = node.children(&mut cursor).collect();
        let mut first = true;

        for (ci, child) in compact_children.iter().enumerate() {
            match child.kind() {
                "{" | "}" => {}
                "," => {
                    // Skip trailing commas (PJF removes them)
                    let has_more_elements = compact_children[ci + 1..]
                        .iter()
                        .any(|c| c.is_named() && !c.is_extra());
                    if has_more_elements {
                        items.push_string(",".to_string());
                        items.extend(helpers::gen_space());
                    }
                }
                _ if child.is_named() => {
                    if first {
                        // No leading space for compact initializers
                    }
                    items.extend(gen_node(*child, context));
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
///
/// PJF wraps at `=` when the RHS is a chain that would fit at continuation indent,
/// preferring `this.field =\n        chain.method()` over `this.field = chain\n        .method()`.
pub fn gen_assignment_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();

    let mut cursor = node.walk();
    let children: Vec<_> = node.children(&mut cursor).collect();

    // Find the LHS, operator, and RHS
    let lhs = node.child_by_field_name("left");
    let rhs = node.child_by_field_name("right");

    // Determine if we should wrap at '='
    let wrap_at_eq = if let (Some(lhs_node), Some(rhs_node)) = (lhs, rhs) {
        let is_chain = rhs_node.kind() == "method_invocation" && chain_depth(rhs_node) >= 1;

        if is_chain {
            let indent_unit = context.config.indent_width as usize;
            let indent_col = context.effective_indent_level() * indent_unit;
            let lhs_text = &context.source[lhs_node.start_byte()..lhs_node.end_byte()];
            let lhs_width = collapse_whitespace(lhs_text).len();

            // Check if chain fits inline at current position (after "LHS = ")
            let current_col = indent_col + lhs_width + 3;
            let chain_fits_current =
                chain_fits_inline_at(rhs_node, current_col, context.source, context.config);

            if !chain_fits_current {
                // Chain would wrap. Check if wrapping at '=' lets the chain stay inline.
                let continuation_col = indent_col + 2 * indent_unit;
                chain_fits_inline_at(rhs_node, continuation_col, context.source, context.config)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    let mut saw_eq = false;
    for child in &children {
        if child.is_named() {
            if wrap_at_eq && saw_eq {
                context.set_assignment_wrapped(true);
                context.add_continuation_indent(2);
            }
            items.extend(gen_node(*child, context));
            if wrap_at_eq && saw_eq {
                context.remove_continuation_indent(2);
                context.set_assignment_wrapped(false);
            }
        } else {
            let op = &context.source[child.start_byte()..child.end_byte()];
            items.extend(helpers::gen_space());
            items.push_string(op.to_string());
            saw_eq = true;
            if wrap_at_eq {
                items.push_signal(Signal::StartIndent);
                items.push_signal(Signal::StartIndent);
                items.push_signal(Signal::NewLine);
            } else {
                items.extend(helpers::gen_space());
            }
        }
    }

    if wrap_at_eq {
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
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
