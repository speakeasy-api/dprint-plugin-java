use dprint_core::formatting::PrintItems;

use super::context::FormattingContext;
use super::declarations;
use super::generate::gen_node;
use super::helpers;

/// Format a binary expression: `a + b`, `x && y`, etc.
pub fn gen_binary_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
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
pub fn gen_method_invocation<'a>(
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
            _ if child.is_named() => {
                items.extend(gen_node(child, context));
            }
            _ => {}
        }
    }

    items
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
pub fn gen_ternary_expression<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

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
pub fn gen_array_initializer<'a>(
    node: tree_sitter::Node<'a>,
    context: &mut FormattingContext<'a>,
) -> PrintItems {
    let mut items = PrintItems::new();
    let mut cursor = node.walk();

    items.push_string("{".to_string());
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
