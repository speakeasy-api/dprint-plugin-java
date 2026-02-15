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
    let content = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
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
        "public class Hello {\n    private int x;\n}\n",
    );
}

#[test]
fn spec_class_bad_indent() {
    run_spec(
        "class_bad_indent",
        "public class Hello {\nprivate int x;\n}\n",
        "public class Hello {\n    private int x;\n}\n",
    );
}

#[test]
fn spec_class_missing_brace_space() {
    run_spec(
        "class_missing_brace_space",
        "public class Hello{\n    private int x;\n}\n",
        "public class Hello {\n    private int x;\n}\n",
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
        "public class Test {\n    public static void main(String[] args) {\n        System.out.println(\"hello\");\n    }\n}\n",
    );
}

#[test]
fn spec_method_bad_indent() {
    run_spec(
        "method_bad_indent",
        "public class Test {\npublic void foo() {\nreturn;\n}\n}\n",
        "public class Test {\n    public void foo() {\n        return;\n    }\n}\n",
    );
}

#[test]
fn spec_constructor() {
    run_spec(
        "constructor",
        "public class Person {\n    private String name;\n\n    public Person(String name) {\n        this.name = name;\n    }\n}\n",
        "public class Person {\n    private String name;\n\n    public Person(String name) {\n        this.name = name;\n    }\n}\n",
    );
}

#[test]
fn spec_interface() {
    run_spec(
        "interface",
        "public interface Printable {\n    void print();\n\n    String toString();\n}\n",
        "public interface Printable {\n    void print();\n\n    String toString();\n}\n",
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
        "public class Test {\n    private int count = 0;\n    private String name = \"default\";\n}\n",
    );
}

// ======== Statement specs ========

#[test]
fn spec_if_else() {
    run_spec(
        "if_else",
        "public class Test {\n    void test() {\n        if (x > 0) {\n            return;\n        } else {\n            throw new RuntimeException();\n        }\n    }\n}\n",
        "public class Test {\n    void test() {\n        if (x > 0) {\n            return;\n        } else {\n            throw new RuntimeException();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_if_else_if() {
    run_spec(
        "if_else_if",
        "public class Test {\n    void test() {\n        if (x > 0) {\n            a();\n        } else if (x < 0) {\n            b();\n        } else {\n            c();\n        }\n    }\n}\n",
        "public class Test {\n    void test() {\n        if (x > 0) {\n            a();\n        } else if (x < 0) {\n            b();\n        } else {\n            c();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_for_loop() {
    run_spec(
        "for_loop",
        "public class Test {\n    void test() {\n        for (int i = 0; i < 10; i++) {\n            doSomething(i);\n        }\n    }\n}\n",
        "public class Test {\n    void test() {\n        for (int i = 0; i < 10; i++) {\n            doSomething(i);\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_enhanced_for() {
    run_spec(
        "enhanced_for",
        "public class Test {\n    void test() {\n        for (String item : items) {\n            process(item);\n        }\n    }\n}\n",
        "public class Test {\n    void test() {\n        for (String item : items) {\n            process(item);\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_while_loop() {
    run_spec(
        "while_loop",
        "public class Test {\n    void test() {\n        while (running) {\n            tick();\n        }\n    }\n}\n",
        "public class Test {\n    void test() {\n        while (running) {\n            tick();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_try_catch_finally() {
    run_spec(
        "try_catch_finally",
        "public class Test {\n    void test() {\n        try {\n            risky();\n        } catch (IOException e) {\n            handle(e);\n        } finally {\n            cleanup();\n        }\n    }\n}\n",
        "public class Test {\n    void test() {\n        try {\n            risky();\n        } catch (IOException e) {\n            handle(e);\n        } finally {\n            cleanup();\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_return_and_throw() {
    run_spec(
        "return_and_throw",
        "public class Test {\n    int test() {\n        if (error) {\n            throw new RuntimeException(\"oops\");\n        }\n        return 42;\n    }\n}\n",
        "public class Test {\n    int test() {\n        if (error) {\n            throw new RuntimeException(\"oops\");\n        }\n        return 42;\n    }\n}\n",
    );
}

#[test]
fn spec_do_while() {
    run_spec(
        "do_while",
        "public class Test {\n    void test() {\n        do {\n            process();\n        } while (hasMore());\n    }\n}\n",
        "public class Test {\n    void test() {\n        do {\n            process();\n        } while (hasMore());\n    }\n}\n",
    );
}

// ======== Expression specs ========

#[test]
fn spec_binary_expression() {
    run_spec(
        "binary_expression",
        "public class Test {\n    void test() {\n        int x = a + b * c;\n    }\n}\n",
        "public class Test {\n    void test() {\n        int x = a + b * c;\n    }\n}\n",
    );
}

#[test]
fn spec_method_chain() {
    run_spec(
        "method_chain",
        "public class Test {\n    void test() {\n        list.stream().filter(x -> x > 0).map(x -> x * 2).collect(Collectors.toList());\n    }\n}\n",
        "public class Test {\n    void test() {\n        list.stream().filter(x -> x > 0).map(x -> x * 2).collect(Collectors.toList());\n    }\n}\n",
    );
}

#[test]
fn spec_lambda() {
    run_spec(
        "lambda",
        "public class Test {\n    void test() {\n        Runnable r = () -> {\n            doSomething();\n        };\n    }\n}\n",
        "public class Test {\n    void test() {\n        Runnable r = () -> {\n            doSomething();\n        };\n    }\n}\n",
    );
}

#[test]
fn spec_ternary() {
    run_spec(
        "ternary",
        "public class Test {\n    void test() {\n        int x = a > b ? a : b;\n    }\n}\n",
        "public class Test {\n    void test() {\n        int x = a > b ? a : b;\n    }\n}\n",
    );
}

#[test]
fn spec_new_object() {
    run_spec(
        "new_object",
        "public class Test {\n    void test() {\n        Object obj = new ArrayList<>(10);\n    }\n}\n",
        "public class Test {\n    void test() {\n        Object obj = new ArrayList<>(10);\n    }\n}\n",
    );
}

#[test]
fn spec_cast_expression() {
    run_spec(
        "cast_expression",
        "public class Test {\n    void test() {\n        String s = (String) obj;\n    }\n}\n",
        "public class Test {\n    void test() {\n        String s = (String) obj;\n    }\n}\n",
    );
}

#[test]
fn spec_instanceof() {
    run_spec(
        "instanceof",
        "public class Test {\n    void test() {\n        if (obj instanceof String) {\n            return;\n        }\n    }\n}\n",
        "public class Test {\n    void test() {\n        if (obj instanceof String) {\n            return;\n        }\n    }\n}\n",
    );
}

#[test]
fn spec_array_access() {
    run_spec(
        "array_access",
        "public class Test {\n    void test() {\n        int x = arr[0];\n        arr[i] = value;\n    }\n}\n",
        "public class Test {\n    void test() {\n        int x = arr[0];\n        arr[i] = value;\n    }\n}\n",
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
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/class_basic.txt"));
}

#[test]
fn spec_file_class_extends() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/class_extends.txt"));
}

#[test]
fn spec_file_class_implements() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/class_implements.txt"));
}

#[test]
fn spec_file_class_modifiers() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/class_modifiers.txt"));
}

#[test]
fn spec_file_class_generic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/class_generic.txt"));
}

#[test]
fn spec_file_class_nested() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/class_nested.txt"));
}

#[test]
fn spec_file_interface_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/interface_basic.txt"));
}

#[test]
fn spec_file_interface_extends() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/interface_extends.txt"));
}

#[test]
fn spec_file_enum_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/enum_basic.txt"));
}

#[test]
fn spec_file_enum_multiple() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/enum_multiple.txt"));
}

#[test]
fn spec_file_method_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/method_basic.txt"));
}

#[test]
fn spec_file_method_params() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/method_params.txt"));
}

#[test]
fn spec_file_method_throws() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/method_throws.txt"));
}

#[test]
fn spec_file_method_generic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/method_generic.txt"));
}

#[test]
fn spec_file_field_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/field_basic.txt"));
}

#[test]
fn spec_file_field_with_init() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/field_with_init.txt"));
}

#[test]
fn spec_file_constructor_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/constructor_basic.txt"));
}

#[test]
fn spec_file_record_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/record_basic.txt"));
}

#[test]
fn spec_file_import_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/import_basic.txt"));
}

#[test]
fn spec_file_package_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/package_basic.txt"));
}

#[test]
fn spec_file_annotation_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/annotation_basic.txt"));
}

#[test]
fn spec_file_varargs() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/varargs.txt"));
}

#[test]
fn spec_file_abstract_class() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/declarations/abstract_class.txt"));
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
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/if_else.txt"));
}

#[test]
fn spec_file_if_else_chain() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/if_else_chain.txt"));
}

#[test]
fn spec_file_for_loop() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/for_loop.txt"));
}

#[test]
fn spec_file_enhanced_for() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/enhanced_for.txt"));
}

#[test]
fn spec_file_while_loop() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/while_loop.txt"));
}

#[test]
fn spec_file_do_while() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/do_while.txt"));
}

#[test]
fn spec_file_switch_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/switch_basic.txt"));
}

#[test]
fn spec_file_try_catch() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/try_catch.txt"));
}

#[test]
fn spec_file_try_with_resources() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/try_with_resources.txt"));
}

#[test]
fn spec_file_return_throw() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/return_throw.txt"));
}

#[test]
fn spec_file_block_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/block_basic.txt"));
}

#[test]
fn spec_file_break_continue() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/break_continue.txt"));
}

#[test]
fn spec_file_synchronized_block() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/synchronized_block.txt"));
}

#[test]
fn spec_file_assert_statement() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/assert_statement.txt"));
}

#[test]
fn spec_file_labeled_statement() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/statements/labeled_statement.txt"));
}

// ---- Expressions ----
#[test]
fn spec_file_binary_ops() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/binary_ops.txt"));
}

#[test]
fn spec_file_method_invocation() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/method_invocation.txt"));
}

#[test]
fn spec_file_lambda_basic() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/lambda_basic.txt"));
}

#[test]
fn spec_file_ternary() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/ternary.txt"));
}

#[test]
fn spec_file_object_creation() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/object_creation.txt"));
}

#[test]
fn spec_file_array_ops() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/array_ops.txt"));
}

#[test]
fn spec_file_cast_instanceof() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/cast_instanceof.txt"));
}

#[test]
fn spec_file_unary_ops() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/unary_ops.txt"));
}

#[test]
fn spec_file_field_access() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/field_access.txt"));
}

#[test]
fn spec_file_parenthesized() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/parenthesized.txt"));
}

#[test]
fn spec_file_assignment() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/assignment.txt"));
}

#[test]
fn spec_file_method_reference() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/expressions/method_reference.txt"));
}

// ---- Mixed/Integration ----
#[test]
fn spec_file_complex_class() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/mixed/complex_class.txt"));
}

#[test]
fn spec_file_bad_formatting() {
    run_spec_file(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/specs/mixed/bad_formatting.txt"));
}
