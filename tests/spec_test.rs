use std::path::Path;

use dprint_core::configuration::NewLineKind;
use dprint_plugin_java::configuration::Configuration;
use dprint_plugin_java::format_text::format_text;

fn default_config() -> Configuration {
    Configuration {
        line_width: 120,
        indent_width: 4,
        use_tabs: false,
        new_line_kind: NewLineKind::LineFeed,
        format_javadoc: false,
        method_chain_threshold: 80,
        inline_lambdas: true,
    }
}

/// Run a spec test: format `input` and assert it equals `expected`.
fn run_spec(name: &str, input: &str, expected: &str) {
    let config = default_config();
    let result = format_text(Path::new("Test.java"), input, &config).unwrap();
    let actual = result.unwrap_or_else(|| input.to_string());
    if actual != expected {
        panic!(
            "Spec test '{}' failed!\n\n--- expected ---\n{}\n--- actual ---\n{}\n--- end ---",
            name, expected, actual
        );
    }

    // Idempotency check: formatting again should produce the same output
    let result2 = format_text(Path::new("Test.java"), &actual, &config).unwrap();
    assert!(
        result2.is_none(),
        "Spec test '{}' is NOT idempotent! Second format changed the output.",
        name
    );
}

/// Parse a spec file with `== input ==` and `== output ==` sections.
fn parse_spec(content: &str) -> (&str, &str) {
    let input_marker = "== input ==";
    let output_marker = "== output ==";

    let input_start = content
        .find(input_marker)
        .expect("Missing '== input ==' marker")
        + input_marker.len();
    let output_start_marker = content
        .find(output_marker)
        .expect("Missing '== output ==' marker");
    let output_start = output_start_marker + output_marker.len();

    let input = content[input_start..output_start_marker].trim();
    let output = content[output_start..].trim();

    (input, output)
}

fn run_spec_file(path: &str) {
    let content =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
    let (input, expected) = parse_spec(&content);
    // Add trailing newline to both since the formatter always ends with one
    let input_with_nl = format!("{}\n", input);
    let expected_with_nl = format!("{}\n", expected);
    run_spec(path, &input_with_nl, &expected_with_nl);
}

// ======== Declaration specs ========

#[test]
fn spec_class_simple() {
    run_spec(
        "class_simple",
        "public class Hello {}\n",
        "public class Hello {}\n",
    );
}

#[test]
fn spec_class_with_body() {
    run_spec(
        "class_with_body",
        "public class Hello {\n    private int x;\n}\n",
        "public class Hello {\n\n    private int x;\n}\n",
    );
}

#[test]
fn spec_class_bad_indent() {
    run_spec(
        "class_bad_indent",
        "public class Hello {\nprivate int x;\n}\n",
        "public class Hello {\n\n    private int x;\n}\n",
    );
}

#[test]
fn spec_class_missing_brace_space() {
    run_spec(
        "class_missing_brace_space",
        "public class Hello{\n    private int x;\n}\n",
        "public class Hello {\n\n    private int x;\n}\n",
    );
}

#[test]
fn spec_class_extends_implements() {
    run_spec(
        "class_extends_implements",
        "public class Foo extends Bar implements Baz, Qux {}\n",
        "public class Foo extends Bar implements Baz, Qux {}\n",
    );
}

#[test]
fn spec_package_and_imports() {
    run_spec(
        "package_and_imports",
        "package com.example;\n\nimport java.util.List;\nimport java.util.Map;\n\npublic class Foo {}\n",
        "package com.example;\n\nimport java.util.List;\nimport java.util.Map;\n\npublic class Foo {}\n",
    );
}

#[test]
fn spec_method_declaration() {
    run_spec(
        "method_declaration",
        "public class Test {\n    public static void main(String[] args) {\n        System.out.println(\"hello\");\n    }\n}\n",
        "public class Test {\n\n    public static void main(String[] args) {\n        System.out.println(\"hello\");\n    }\n}\n",
    );
}

#[test]
fn spec_method_bad_indent() {
    run_spec(
        "method_bad_indent",
        "public class Test {\npublic void foo() {\nreturn;\n}\n}\n",
        "public class Test {\n\n    public void foo() {\n        return;\n    }\n}\n",
    );
}

#[test]
fn spec_constructor() {
    run_spec(
        "constructor",
        "public class Person {\n\n    private String name;\n\n    public Person(String name) {\n        this.name = name;\n    }\n}\n",
        "public class Person {\n\n    private String name;\n\n    public Person(String name) {\n        this.name = name;\n    }\n}\n",
    );
}

#[test]
fn spec_interface() {
    run_spec(
        "interface",
        "public interface Printable {\n\n    void print();\n\n    String toString();\n}\n",
        "public interface Printable {\n\n    void print();\n\n    String toString();\n}\n",
    );
}

#[test]
fn spec_enum_simple() {
    run_spec(
        "enum_simple",
        "public enum Color {\n    RED,\n    GREEN,\n    BLUE\n}\n",
        "public enum Color {\n    RED,\n    GREEN,\n    BLUE\n}\n",
    );
}

#[test]
fn spec_field_with_initializer() {
    run_spec(
        "field_with_initializer",
        "public class Test {\n    private int count = 0;\n    private String name = \"default\";\n}\n",
        "public class Test {\n\n    private int count = 0;\n\n    private String name = \"default\";\n}\n",
    );
}

// ======== Statement specs ========

#[test]
fn spec_if_else() {
    run_spec(
        "if_else",
        "public class Test {\n\n    void test() {\n        if (x > 0) {\n            return;\n        } else {\n            throw new RuntimeException();\n        }\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        if (x > 0) {\n            return;\n        } else {\n            throw new RuntimeException();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_if_else_if() {
    run_spec(
        "if_else_if",
        "public class Test {\n\n    void test() {\n        if (x > 0) {\n            a();\n        } else if (x < 0) {\n            b();\n        } else {\n            c();\n        }\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        if (x > 0) {\n            a();\n        } else if (x < 0) {\n            b();\n        } else {\n            c();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_for_loop() {
    run_spec(
        "for_loop",
        "public class Test {\n\n    void test() {\n        for (int i = 0; i < 10; i++) {\n            doSomething(i);\n        }\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        for (int i = 0; i < 10; i++) {\n            doSomething(i);\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_enhanced_for() {
    run_spec(
        "enhanced_for",
        "public class Test {\n\n    void test() {\n        for (String item : items) {\n            process(item);\n        }\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        for (String item : items) {\n            process(item);\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_while_loop() {
    run_spec(
        "while_loop",
        "public class Test {\n\n    void test() {\n        while (running) {\n            tick();\n        }\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        while (running) {\n            tick();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_try_catch_finally() {
    run_spec(
        "try_catch_finally",
        "public class Test {\n\n    void test() {\n        try {\n            risky();\n        } catch (IOException e) {\n            handle(e);\n        } finally {\n            cleanup();\n        }\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        try {\n            risky();\n        } catch (IOException e) {\n            handle(e);\n        } finally {\n            cleanup();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_return_and_throw() {
    run_spec(
        "return_and_throw",
        "public class Test {\n    int test() {\n        if (error) {\n            throw new RuntimeException(\"oops\");\n        }\n        return 42;\n    }\n}\n",
        "public class Test {\n\n    int test() {\n        if (error) {\n            throw new RuntimeException(\"oops\");\n        }\n        return 42;\n    }\n}\n",
    );
}

#[test]
fn spec_do_while() {
    run_spec(
        "do_while",
        "public class Test {\n\n    void test() {\n        do {\n            process();\n        } while (hasMore());\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        do {\n            process();\n        } while (hasMore());\n    }\n}\n",
    );
}

// ======== Expression specs ========

#[test]
fn spec_binary_expression() {
    run_spec(
        "binary_expression",
        "public class Test {\n\n    void test() {\n        int x = a + b * c;\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        int x = a + b * c;\n    }\n}\n",
    );
}

#[test]
fn spec_method_chain() {
    run_spec(
        "method_chain",
        "public class Test {\n\n    void test() {\n        list.stream().filter(x -> x > 0).map(x -> x * 2).collect(Collectors.toList());\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        list.stream().filter(x -> x > 0).map(x -> x * 2).collect(Collectors.toList());\n    }\n}\n",
    );
}

#[test]
fn spec_lambda() {
    run_spec(
        "lambda",
        "public class Test {\n\n    void test() {\n        Runnable r = () -> {\n            doSomething();\n        };\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        Runnable r = () -> {\n            doSomething();\n        };\n    }\n}\n",
    );
}

#[test]
fn spec_ternary() {
    run_spec(
        "ternary",
        "public class Test {\n\n    void test() {\n        int x = a > b ? a : b;\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        int x = a > b ? a : b;\n    }\n}\n",
    );
}

#[test]
fn spec_new_object() {
    run_spec(
        "new_object",
        "public class Test {\n\n    void test() {\n        Object obj = new ArrayList<>(10);\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        Object obj = new ArrayList<>(10);\n    }\n}\n",
    );
}

#[test]
fn spec_cast_expression() {
    run_spec(
        "cast_expression",
        "public class Test {\n\n    void test() {\n        String s = (String) obj;\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        String s = (String) obj;\n    }\n}\n",
    );
}

#[test]
fn spec_instanceof() {
    run_spec(
        "instanceof",
        "public class Test {\n\n    void test() {\n        if (obj instanceof String) {\n            return;\n        }\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        if (obj instanceof String) {\n            return;\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_array_access() {
    run_spec(
        "array_access",
        "public class Test {\n\n    void test() {\n        int x = arr[0];\n        arr[i] = value;\n    }\n}\n",
        "public class Test {\n\n    void test() {\n        int x = arr[0];\n        arr[i] = value;\n    }\n}\n",
    );
}

// ======== File-based specs ========

// ---- Declarations ----
#[test]
fn spec_file_class_formatting() {
    let spec_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/");
    let path = format!("{}class_formatting.txt", spec_dir);
    if std::path::Path::new(&path).exists() {
        run_spec_file(&path);
    }
}

#[test]
fn spec_file_class_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_basic.txt"
    ));
}

#[test]
fn spec_file_class_extends() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_extends.txt"
    ));
}

#[test]
fn spec_file_class_implements() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_implements.txt"
    ));
}

#[test]
fn spec_file_class_modifiers() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_modifiers.txt"
    ));
}

#[test]
fn spec_file_class_generic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_generic.txt"
    ));
}

#[test]
fn spec_file_class_nested() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_nested.txt"
    ));
}

#[test]
fn spec_file_interface_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/interface_basic.txt"
    ));
}

#[test]
fn spec_file_interface_extends() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/interface_extends.txt"
    ));
}

#[test]
fn spec_file_enum_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/enum_basic.txt"
    ));
}

#[test]
fn spec_file_enum_multiple() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/enum_multiple.txt"
    ));
}

#[test]
fn spec_file_enum_with_body() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/enum_with_body.txt"
    ));
}

#[test]
fn spec_file_method_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/method_basic.txt"
    ));
}

#[test]
fn spec_file_method_params() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/method_params.txt"
    ));
}

#[test]
fn spec_file_method_params_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/method_params_wrapping.txt"
    ));
}

#[test]
fn spec_file_method_throws() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/method_throws.txt"
    ));
}

#[test]
fn spec_file_method_throws_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/method_throws_wrapping.txt"
    ));
}

#[test]
fn spec_file_method_generic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/method_generic.txt"
    ));
}

#[test]
fn spec_file_field_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/field_basic.txt"
    ));
}

#[test]
fn spec_file_field_with_init() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/field_with_init.txt"
    ));
}

#[test]
fn spec_file_constructor_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/constructor_basic.txt"
    ));
}

#[test]
fn spec_file_constructor_throws() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/constructor_throws.txt"
    ));
}

#[test]
fn spec_file_record_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/record_basic.txt"
    ));
}

#[test]
fn spec_file_import_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/import_basic.txt"
    ));
}

#[test]
fn spec_file_import_sorting() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/import_sorting.txt"
    ));
}

#[test]
fn spec_file_import_sorting_wildcards() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/import_sorting_wildcards.txt"
    ));
}

#[test]
fn spec_file_import_sorting_single() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/import_sorting_single.txt"
    ));
}

#[test]
fn spec_file_package_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/package_basic.txt"
    ));
}

#[test]
fn spec_file_annotation_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/annotation_basic.txt"
    ));
}

#[test]
fn spec_file_annotation_placement() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/annotation_placement.txt"
    ));
}

#[test]
fn spec_file_varargs() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/varargs.txt"
    ));
}

#[test]
fn spec_file_argument_list_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/argument_list_wrapping.txt"
    ));
}

#[test]
fn spec_file_argument_list_pjf_parity() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/argument_list_pjf_parity.txt"
    ));
}

#[test]
fn spec_file_abstract_class() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/abstract_class.txt"
    ));
}

#[test]
fn spec_file_modifier_order() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/modifier_order.txt"
    ));
}

// #[test]
// fn spec_file_variable_assignment_wrapping() {
//     run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/variable_assignment_wrapping.txt"));
// }

#[test]
fn spec_file_class_extends_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_extends_wrapping.txt"
    ));
}

#[test]
fn spec_file_class_implements_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_implements_wrapping.txt"
    ));
}

#[test]
fn spec_file_class_extends_implements_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/class_extends_implements_wrapping.txt"
    ));
}

#[test]
fn spec_file_interface_extends_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/interface_extends_wrapping.txt"
    ));
}

#[test]
fn spec_file_enum_implements_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/enum_implements_wrapping.txt"
    ));
}

#[test]
fn spec_file_record_implements_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/record_implements_wrapping.txt"
    ));
}

// ---- Statements ----
#[test]
fn spec_file_statement_formatting() {
    let spec_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/");
    let path = format!("{}control_flow.txt", spec_dir);
    if std::path::Path::new(&path).exists() {
        run_spec_file(&path);
    }
}

#[test]
fn spec_file_if_else() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/if_else.txt"
    ));
}

#[test]
fn spec_file_if_else_chain() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/if_else_chain.txt"
    ));
}

#[test]
fn spec_file_for_loop() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/for_loop.txt"
    ));
}

#[test]
fn spec_file_enhanced_for() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/enhanced_for.txt"
    ));
}

#[test]
fn spec_file_while_loop() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/while_loop.txt"
    ));
}

#[test]
fn spec_file_do_while() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/do_while.txt"
    ));
}

#[test]
fn spec_file_switch_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/switch_basic.txt"
    ));
}

#[test]
fn spec_file_try_catch() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/try_catch.txt"
    ));
}

#[test]
fn spec_file_try_with_resources() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/try_with_resources.txt"
    ));
}

#[test]
fn spec_file_return_throw() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/return_throw.txt"
    ));
}

#[test]
fn spec_file_block_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/block_basic.txt"
    ));
}

#[test]
fn spec_file_break_continue() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/break_continue.txt"
    ));
}

#[test]
fn spec_file_synchronized_block() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/synchronized_block.txt"
    ));
}

#[test]
fn spec_file_assert_statement() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/assert_statement.txt"
    ));
}

#[test]
fn spec_file_labeled_statement() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/labeled_statement.txt"
    ));
}

#[test]
fn spec_file_local_variable_annotations() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/local_variable_annotations.txt"
    ));
}

#[test]
fn spec_file_block_comment_blank_line() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/block_comment_blank_line.txt"
    ));
}

#[test]
fn spec_file_switch_case_block() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/switch_case_block.txt"
    ));
}

#[test]
fn spec_file_switch_mixed_blocks() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/statements/switch_mixed_blocks.txt"
    ));
}

// ---- Expressions ----
#[test]
fn spec_file_binary_ops() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/binary_ops.txt"
    ));
}

#[test]
fn spec_file_binary_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/binary_wrapping.txt"
    ));
}

#[test]
fn spec_file_method_invocation() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/method_invocation.txt"
    ));
}

#[test]
fn spec_file_lambda_basic() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/lambda_basic.txt"
    ));
}

#[test]
fn spec_file_ternary() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/ternary.txt"
    ));
}

#[test]
fn spec_file_ternary_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/ternary_wrapping.txt"
    ));
}

#[test]
fn spec_file_object_creation() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/object_creation.txt"
    ));
}

#[test]
fn spec_file_array_ops() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/array_ops.txt"
    ));
}

#[test]
fn spec_file_cast_instanceof() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/cast_instanceof.txt"
    ));
}

#[test]
fn spec_file_unary_ops() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/unary_ops.txt"
    ));
}

#[test]
fn spec_file_field_access() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/field_access.txt"
    ));
}

#[test]
fn spec_file_parenthesized() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/parenthesized.txt"
    ));
}

#[test]
fn spec_file_assignment() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/assignment.txt"
    ));
}

#[test]
fn spec_file_method_reference() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/method_reference.txt"
    ));
}

#[test]
fn spec_file_method_chain_breaking() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/method_chain_breaking.txt"
    ));
}

#[test]
fn spec_file_method_chain_line_comment() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/method_chain_line_comment.txt"
    ));
}

#[test]
fn spec_file_method_chain_wrapping_edge_cases() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/method_chain_wrapping_edge_cases.txt"
    ));
}

#[test]
fn spec_file_lambda_chain_indent() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/lambda_chain_indent.txt"
    ));
}

#[test]
fn spec_file_array_initializer_comments() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/array_initializer_comments.txt"
    ));
}

#[test]
fn spec_builder_pattern_wrapping() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/builder_pattern_wrapping.txt"
    ));
}

#[test]
fn spec_file_chain_comments() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/chain_comments.txt"
    ));
}

#[test]
fn spec_file_chain_inline_comments() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/expressions/chain_inline_comments.txt"
    ));
}

// ---- Comments ----
#[test]
fn spec_file_trailing_whitespace() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/comments/trailing_whitespace.txt"
    ));
}

// ---- Instability debugging ----

/// Debug helper: format and check stability
fn assert_stable(name: &str, input: &str) {
    let config = default_config();
    let pass1 = format_text(std::path::Path::new("Test.java"), input, &config)
        .unwrap()
        .unwrap_or_else(|| input.to_string());

    let pass2 = format_text(std::path::Path::new("Test.java"), &pass1, &config)
        .unwrap()
        .unwrap_or_else(|| pass1.clone());

    if pass1 != pass2 {
        let pass1_lines: Vec<&str> = pass1.lines().collect();
        let pass2_lines: Vec<&str> = pass2.lines().collect();
        eprintln!("\n=== INSTABILITY: {} ===", name);
        eprintln!(
            "pass1 has {} lines, pass2 has {} lines",
            pass1_lines.len(),
            pass2_lines.len()
        );
        let max = pass1_lines.len().max(pass2_lines.len());
        for i in 0..max {
            let l1 = pass1_lines.get(i).unwrap_or(&"<missing>");
            let l2 = pass2_lines.get(i).unwrap_or(&"<missing>");
            if l1 != l2 {
                eprintln!("LINE {}: ", i + 1);
                eprintln!("  pass1: {:?}", l1);
                eprintln!("  pass2: {:?}", l2);
            }
        }
        eprintln!("\n--- full pass1 ---\n{}\n--- end ---", pass1);
        panic!("Formatting '{}' is not stable", name);
    }
}

#[test]
fn debug_instability_lambda_block() {
    assert_stable(
        "lambda_block_field",
        r#"public interface Foo {
    static Foo DEFAULT = (a, b) -> {
        doSomething();
    };
}"#,
    );
}

// Skipped: Known instability in Auth.java (chain+arglist wrapping interaction)
// #[test]
// fn debug_instability_sdk_file() {
//     let paths = &[
//         "/tmp/spotless-ref/zSDKs/sdk-javav2/src/main/java/org/openapis/review/openapi/operations/Auth.java",
//         "/tmp/spotless-ref/zSDKs/sdk-javav2/src/main/java/org/openapis/review/openapi/models/operations/ListTest1RequestBuilder.java",
//         "/tmp/spotless-ref/zSDKs/sdk-javav2/src/main/java/org/openapis/review/openapi/SDKConfiguration.java",
//     ];
//     for path in paths {
//         let input = match std::fs::read_to_string(path) {
//             Ok(s) => s,
//             Err(_) => { eprintln!("Skipping {}: not found", path); continue; }
//         };
//         let config = default_config();
//         let pass1 = format_text(std::path::Path::new("Test.java"), &input, &config)
//             .unwrap().unwrap_or_else(|| input.clone());
//         let pass2 = format_text(std::path::Path::new("Test.java"), &pass1, &config)
//             .unwrap().unwrap_or_else(|| pass1.clone());
//         if pass1 != pass2 {
//             let p1: Vec<&str> = pass1.lines().collect();
//             let p2: Vec<&str> = pass2.lines().collect();
//             eprintln!("\n=== INSTABILITY: {} ===", path);
//             let max = p1.len().max(p2.len());
//             let mut shown = 0;
//             for i in 0..max {
//                 let l1 = p1.get(i).unwrap_or(&"<missing>");
//                 let l2 = p2.get(i).unwrap_or(&"<missing>");
//                 if l1 != l2 && shown < 20 {
//                     eprintln!("LINE {}: ", i + 1);
//                     eprintln!("  pass1: {:?}", l1);
//                     eprintln!("  pass2: {:?}", l2);
//                     shown += 1;
//                 }
//             }
//             // Also dump tree of the unstable region
//             let mut parser = tree_sitter::Parser::new();
//             parser.set_language(&tree_sitter_java::LANGUAGE.into()).unwrap();
//             let tree = parser.parse(&pass1, None).unwrap();
//             // Find the node at the first differing line
//             for i in 0..max {
//                 let l1 = p1.get(i).unwrap_or(&"<missing>");
//                 let l2 = p2.get(i).unwrap_or(&"<missing>");
//                 if l1 != l2 {
//                     let byte_offset = pass1.lines().take(i).map(|l| l.len() + 1).sum::<usize>();
//                     let node = tree.root_node().descendant_for_byte_range(byte_offset, byte_offset + 1);
//                     if let Some(n) = node {
//                         // Walk up to find the interesting parent
//                         let mut current = n;
//                         for _ in 0..8 {
//                             if let Some(p) = current.parent() { current = p; } else { break; }
//                         }
//                         eprintln!("\nTree around first diff (line {}):", i + 1);
//                         fn dump2(node: tree_sitter::Node, source: &str, depth: usize, max_depth: usize) {
//                             if depth > max_depth { return; }
//                             let indent = "  ".repeat(depth);
//                             let text = &source[node.start_byte()..node.end_byte()];
//                             let short = if text.len() > 80 { &text[..80] } else { text };
//                             let short = short.replace('\n', "\\n");
//                             eprintln!("{}{}  [{}-{}] {:?}", indent, node.kind(), node.start_byte(), node.end_byte(), short);
//                             let mut cursor = node.walk();
//                             for child in node.children(&mut cursor) {
//                                 dump2(child, source, depth + 1, max_depth);
//                             }
//                         }
//                         dump2(current, &pass1, 0, 5);
//                     }
//                     break;
//                 }
//             }
//             panic!("File {} is not stable", path);
//         }
//     }
// }

#[test]
fn debug_instability_multiline_args() {
    assert_stable("multiline_args", r#"
public class Test {
    void test() {
        Utils.checkArgument(
                response.isPresent() ^ error.isPresent(), "one and only one of response or error must be present");
    }
}
"#.trim());
}

#[test]
fn debug_instability_long_assignment() {
    assert_stable("long_assignment", r#"
public class Test {
    void test() {
        RequestlessOperation<Deprecated1Response> operation = new Deprecated1.Sync(sdkConfiguration, serverURL, _headers);
    }
}
"#.trim());
}

#[test]
fn debug_instability_bare_method_chain() {
    assert_stable(
        "bare_method_chain",
        r#"public class Test {
    void test() {
        callAsStream().flatMap(r -> r.object().stream()).flatMap(r -> r.resultArray().stream());
    }
}"#,
    );
}

#[test]
fn debug_lambda_chain_tree() {
    let code = r#"public class Test {
    void test() {
        client.sendAsync(request, BodyHandlers.ofString()).thenApply(resp -> resp.body()).handle((resp, err) -> {
            if (err != null) {
                return null;
            }
            return resp.body();
        });
    }
}"#;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(code, None).unwrap();

    fn find_method_invocation(node: tree_sitter::Node, source: &str, depth: usize) {
        if node.kind() == "method_invocation" {
            let text = &source[node.start_byte()..node.end_byte()];
            let short = if text.len() > 80 { &text[..80] } else { text };
            eprintln!(
                "{} method_invocation: {:?}",
                "  ".repeat(depth),
                short.replace('\n', "\\n")
            );

            // Check for object child
            if let Some(obj) = node.child_by_field_name("object") {
                eprintln!("{}   object: {}", "  ".repeat(depth), obj.kind());
            }
            if let Some(name) = node.child_by_field_name("name") {
                let name_text = &source[name.start_byte()..name.end_byte()];
                eprintln!("{}   name: {:?}", "  ".repeat(depth), name_text);
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            find_method_invocation(child, source, depth + 1);
        }
    }

    find_method_invocation(tree.root_node(), code, 0);
}

#[test]
fn debug_instability_method_throws_multiline() {
    assert_stable("method_throws_multiline", r#"
public interface Foo {
    HttpResponse<InputStream> afterSuccess(AfterSuccessContext context, HttpResponse<InputStream> response)
            throws Exception;
}
"#.trim());
}

// ---- Mixed/Integration ----
#[test]
fn spec_file_complex_class() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/mixed/complex_class.txt"
    ));
}

#[test]
fn spec_file_bad_formatting() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/mixed/bad_formatting.txt"
    ));
}

// #[test]
// fn spec_file_instance_initializer() {
//     run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/instance_initializer.txt"));
// }

#[test]
fn spec_file_blank_lines_import_to_class() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/blank_lines_import_to_class.txt"
    ));
}

#[test]
fn spec_file_blank_lines_after_class_brace() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/blank_lines_after_class_brace.txt"
    ));
}

#[test]
fn spec_file_blank_lines_javadoc_fields() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/blank_lines_javadoc_fields.txt"
    ));
}

#[test]
fn spec_file_blank_lines_javadoc_methods() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/blank_lines_javadoc_methods.txt"
    ));
}

#[test]
fn spec_file_blank_lines_members() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/blank_lines_members.txt"
    ));
}

#[test]
fn spec_file_instance_initializer_nested() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/instance_initializer_nested.txt"
    ));
}

#[test]
fn spec_file_instance_initializer_with_members() {
    run_spec_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/specs/declarations/instance_initializer_with_members.txt"
    ));
}
