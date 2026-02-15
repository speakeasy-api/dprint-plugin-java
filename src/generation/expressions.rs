use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use super::context::FormattingContext;
use super::declarations;
use super::generate::gen_node;
use super::helpers;

/// Collapse whitespace in a string: replace newlines and multiple spaces with single spaces.
/// This helps estimate the "flat" width of a code fragment as if formatted on one line.
fn collapse_whitespace(s: &str) -> String {
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

/// Format a binary expression: `a + b`, `x && y`, etc.
///
/// For long chains of `&&` or `||` operators, wraps before each operator
/// with 8-space continuation indent (PJF style):
/// ```java
/// return Utils.enhancedDeepEquals(this.contentType, other.contentType)
///         && Utils.enhancedDeepEquals(this.statusCode, other.statusCode)
///         && Utils.enhancedDeepEquals(this.rawResponse, other.rawResponse);
/// ```
pub fn gen_binary_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    // Get the operator of this binary expression
    let mut cursor = node.walk();
    let operator = node
        .children(&mut cursor)
        .find(|c| !c.is_named())
        .map(|c| context.source[c.start_byte()..c.end_byte()].to_string());

    // Check if this is a logical operator (&& or ||)
    let is_logical_op = matches!(operator.as_deref(), Some("&&") | Some("||"));

    if is_logical_op {
        // Check if this node is the RIGHT child of a parent binary_expression with && or ||
        // If so, we're nested and should let the parent handle the whole chain
        let is_nested_in_chain = if let Some(parent) = node.parent() {
            if parent.kind() == "binary_expression" {
                // Check if we're the right child
                let parent_children: Vec<_> = parent.children(&mut parent.walk()).collect();
                let right_child = parent_children.iter().rev().find(|c| c.is_named());
                if let Some(right) = right_child {
                    if right.id() == node.id() {
                        // We're the right child, check if parent has && or ||
                        let parent_op = parent_children
                            .iter()
                            .find(|c| !c.is_named())
                            .map(|c| context.source[c.start_byte()..c.end_byte()].to_string());
                        matches!(parent_op.as_deref(), Some("&&") | Some("||"))
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
            // We're at the root of the chain
            // Flatten the chain and collect all operands and operators
            let (operands, operators) = flatten_logical_chain(node, context.source);

            // Estimate the flat width of the entire expression.
            // For a variable declaration, we need to check the full line including the prefix.
            // Walk up to find if we're inside a statement that spans from the start of the line.
            let should_wrap = {
                let indent_width = context.indent_level() * context.config.indent_width as usize;

                // Find the start of the line by looking for the containing statement
                let line_start_byte = if let Some(parent) = node.parent() {
                    // Check if parent is a variable_declarator
                    if parent.kind() == "variable_declarator" {
                        // Go up to local_variable_declaration or field_declaration
                        if let Some(grandparent) = parent.parent() {
                            match grandparent.kind() {
                                "local_variable_declaration" | "field_declaration" => grandparent.start_byte(),
                                _ => node.start_byte(),
                            }
                        } else {
                            node.start_byte()
                        }
                    } else {
                        node.start_byte()
                    }
                } else {
                    node.start_byte()
                };

                let line_text = &context.source[line_start_byte..node.end_byte()];
                let line_flat_width: usize = line_text
                    .lines()
                    .map(|l| l.trim().len())
                    .sum::<usize>()
                    + line_text.lines().count().saturating_sub(1);

                indent_width + line_flat_width > context.config.line_width as usize
            };

            if should_wrap {
                // Wrapped: break before each && or || with 8-space continuation indent
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
            // Operator token
            let op = &context.source[child.start_byte()..child.end_byte()];
            items.extend(helpers::gen_space());
            items.push_string(op.to_string());
            items.extend(helpers::gen_space());
        }
    }

    items
}

/// Flatten a chain of binary expressions with && or || operators.
/// Returns (operands, operators) where operands[i] op operators[i] = operands[i+1].
fn flatten_logical_chain<'a>(
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

        // Get operator
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        let op = children
            .iter()
            .find(|c| !c.is_named())
            .map(|c| source[c.start_byte()..c.end_byte()].to_string());

        // Only flatten if it's && or ||
        if !matches!(op.as_deref(), Some("&&") | Some("||")) {
            operands.push(node);
            return;
        }

        // Get left and right operands
        let left = children.iter().find(|c| c.is_named()).unwrap();
        let right = children.iter().rev().find(|c| c.is_named()).unwrap();

        // Recursively collect left side
        collect(*left, source, operands, operators);

        // Add this operator
        operators.push(op.unwrap());

        // Recursively collect right side
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
/// chain and checks if the flat width exceeds `method_chain_threshold`. If so,
/// it breaks the line before each `.method()` with 8-space continuation indent.
pub fn gen_method_invocation<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let depth = chain_depth(node);
    if depth < 2 {
        return gen_method_invocation_simple(node, context);
    }

    // Flatten the chain into (root, [(method_invocation_node, method_name_node, type_args, arg_list), ...])
    let mut segments: Vec<(tree_sitter::Node<'a>, tree_sitter::Node<'a>, Option<tree_sitter::Node<'a>>, Option<tree_sitter::Node<'a>>)> = Vec::new();
    let root = flatten_chain(node, &mut segments);

    // Force wrapping if any segment has a lambda with a block body
    let force_wrap = chain_has_lambda_block(&segments);

    // Calculate the flat width by estimating the formatted width of each component.
    // We compute this as the text length with newlines/multi-space runs collapsed to single spaces.
    let root_text = &context.source[root.start_byte()..root.end_byte()];
    let root_width = collapse_whitespace(root_text).len();

    // Sum up each segment: . + name + type_args + arg_list
    let mut segments_width = 0;
    for (_, name_node, type_args, arg_list) in &segments {
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
    }

    let chain_flat_width = root_width + segments_width;
    let should_wrap = force_wrap || chain_flat_width > context.config.method_chain_threshold as usize;

    let mut items = PrintItems::new();
    items.extend(gen_node(root, context));

    if should_wrap {
        // Force line breaks with 8-space continuation indent (2x indent_width)
        items.push_signal(Signal::StartIndent);
        items.push_signal(Signal::StartIndent);
        for (_, name_node, type_args, arg_list) in segments {
            items.push_signal(Signal::NewLine);
            items.push_string(".".to_string());
            if let Some(ta) = type_args {
                items.extend(gen_node(ta, context));
            }
            items.extend(helpers::gen_node_text(name_node, context.source));
            if let Some(al) = arg_list {
                items.extend(declarations::gen_argument_list(al, context));
            }
        }
        items.push_signal(Signal::FinishIndent);
        items.push_signal(Signal::FinishIndent);
    } else {
        // Keep on one line
        for (_, name_node, type_args, arg_list) in segments {
            items.push_string(".".to_string());
            if let Some(ta) = type_args {
                items.extend(gen_node(ta, context));
            }
            items.extend(helpers::gen_node_text(name_node, context.source));
            if let Some(al) = arg_list {
                items.extend(declarations::gen_argument_list(al, context));
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
                items.extend(declarations::gen_argument_list(child, context));
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
fn chain_has_lambda_block(
    segments: &[(tree_sitter::Node, tree_sitter::Node, Option<tree_sitter::Node>, Option<tree_sitter::Node>)],
) -> bool {
    for (_, _, _, arg_list) in segments {
        if let Some(al) = arg_list {
            if arg_list_has_lambda_block(*al) {
                return true;
            }
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

/// Count how deep a method invocation chain is.
/// `a.b()` = 1, `a.b().c()` = 2, `a.b().c().d()` = 3, etc.
fn chain_depth(node: tree_sitter::Node) -> usize {
    let mut depth = 0;
    let mut current = node;
    loop {
        let mut cursor = current.walk();
        let object = current.children(&mut cursor).find(|c| {
            c.is_named() && c.kind() != "argument_list" && c.kind() != "type_arguments"
        });
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
fn flatten_chain<'a>(
    node: tree_sitter::Node<'a>,
    segments: &mut Vec<(tree_sitter::Node<'a>, tree_sitter::Node<'a>, Option<tree_sitter::Node<'a>>, Option<tree_sitter::Node<'a>>)>,
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
            current.children(&mut cursor).find(|c| c.kind() == "type_arguments")
        };
        let arg_list = current.child_by_field_name("arguments");

        if let Some(name_node) = name {
            chain.push((current, name_node, type_args, arg_list));
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
    let ternary_flat_width: usize = ternary_text
        .lines()
        .map(|l| l.trim().len())
        .sum::<usize>()
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
                items.extend(declarations::gen_argument_list(child, context));
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
pub fn gen_array_initializer<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    // Check if this array initializer has any comments
    let has_comments = node.children(&mut cursor).any(|c| c.is_extra());

    // Reset cursor for iteration
    cursor = node.walk();

    items.push_string("{".to_string());

    if has_comments {
        // Expanded format: one element per line
        items.push_signal(Signal::StartIndent);
        let mut prev_was_line_comment = false;

        for child in node.children(&mut cursor) {
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
                    items.extend(gen_node(child, context));
                    prev_was_line_comment = child.kind() == "line_comment";
                }
                _ if child.is_named() => {
                    // Element node
                    if !prev_was_line_comment {
                        items.push_signal(Signal::NewLine);
                    }
                    items.extend(gen_node(child, context));
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
            "argument_list" => items.extend(declarations::gen_argument_list(child, context)),
            ";" => items.push_string(";".to_string()),
            "type_arguments" => items.extend(gen_node(child, context)),
            _ if child.is_named() => items.extend(gen_node(child, context)),
            _ => {}
        }
    }

    items
}
