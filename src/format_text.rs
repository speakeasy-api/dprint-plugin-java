use std::path::Path;

use anyhow::Result;
use dprint_core::configuration::resolve_new_line_kind;
use dprint_core::formatting::PrintOptions;

use crate::configuration::Configuration;
use crate::generation::generate;

/// Format a Java source file. Returns `Ok(None)` if no changes were made.
///
/// # Errors
///
/// Returns an error if the source cannot be parsed or formatted.
pub fn format_text(
    _file_path: &Path,
    file_text: &str,
    config: &Configuration,
) -> Result<Option<String>> {
    let formatted = format_text_inner(file_text, config)?;
    if formatted == file_text {
        Ok(None)
    } else {
        Ok(Some(formatted))
    }
}

fn format_text_inner(file_text: &str, config: &Configuration) -> Result<String> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|e| anyhow::anyhow!("Failed to load Java grammar: {e}"))?;

    let tree = parser
        .parse(file_text, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse Java source"))?;

    if tree.root_node().has_error() {
        // For now, return the source unchanged if there are parse errors.
        // A production formatter might still attempt best-effort formatting.
        return Ok(file_text.to_string());
    }

    let print_items = generate(file_text, &tree, config);
    let print_options = build_print_options(file_text, config);

    Ok(dprint_core::formatting::format(
        || print_items,
        print_options,
    ))
}

fn build_print_options(file_text: &str, config: &Configuration) -> PrintOptions {
    PrintOptions {
        indent_width: config.indent_width,
        max_width: config.line_width,
        use_tabs: config.use_tabs,
        new_line_text: resolve_new_line_kind(file_text, config.new_line_kind),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::Configuration;
    use dprint_core::configuration::NewLineKind;

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

    #[test]
    fn formats_simple_class() {
        let input = "public class Hello {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, world!\");\n    }\n}\n";
        let result = format_text(Path::new("Hello.java"), input, &default_config()).unwrap();
        // With formatting enabled, the output should be well-formatted
        // If None, input was already correctly formatted
        match result {
            Some(formatted) => {
                assert!(formatted.contains("public class Hello"));
                assert!(formatted.contains("public static void main"));
            }
            None => {
                // Already correctly formatted - that's fine
            }
        }
    }

    #[test]
    fn handles_parse_error_gracefully() {
        let input = "public class { broken syntax";
        let result = format_text(Path::new("Bad.java"), input, &default_config()).unwrap();
        // Should return None (unchanged) for parse errors
        assert!(result.is_none());
    }

    #[test]
    fn formats_package_and_imports() {
        let input = "package com.example;\nimport java.util.List;\nimport java.util.Map;\npublic class Foo {}\n";
        let result = format_text(Path::new("Foo.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.starts_with("package com.example;"));
        assert!(output.contains("import java.util.List;"));
        assert!(output.contains("import java.util.Map;"));
        assert!(output.contains("public class Foo {}"));
    }

    #[test]
    fn formats_class_with_fields_and_methods() {
        let input = "public class Person {\n    private String name;\n    private int age;\n\n    public Person(String name, int age) {\n        this.name = name;\n        this.age = age;\n    }\n\n    public String getName() {\n        return name;\n    }\n}\n";
        let result = format_text(Path::new("Person.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("private String name;"));
        assert!(output.contains("public Person(String name, int age)"));
        assert!(output.contains("return name;"));
    }

    #[test]
    fn formats_if_else() {
        let input = "public class Test {\n    void test() {\n        if (x > 0) {\n            return;\n        } else {\n            throw new RuntimeException();\n        }\n    }\n}\n";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("if (x > 0)"));
        assert!(output.contains("} else {"));
    }

    #[test]
    fn formats_enum() {
        let input = "public enum Color {\n    RED,\n    GREEN,\n    BLUE\n}\n";
        let result = format_text(Path::new("Color.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("public enum Color"));
        assert!(output.contains("RED"));
        assert!(output.contains("GREEN"));
        assert!(output.contains("BLUE"));
    }

    #[test]
    fn formats_try_catch() {
        let input = "public class Test {\n    void test() {\n        try {\n            doSomething();\n        } catch (Exception e) {\n            handleError(e);\n        } finally {\n            cleanup();\n        }\n    }\n}\n";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("try {"));
        assert!(output.contains("} catch (Exception e) {"));
        assert!(output.contains("} finally {"));
    }

    #[test]
    fn formats_for_loop() {
        let input = "public class Test {\n    void test() {\n        for (int i = 0; i < 10; i++) {\n            doSomething(i);\n        }\n    }\n}\n";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("for ("));
        assert!(output.contains("doSomething(i);"));
    }

    #[test]
    fn formats_interface() {
        let input = "public interface Printable {\n    void print();\n}\n";
        let result = format_text(Path::new("Printable.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("public interface Printable"));
        assert!(output.contains("void print();"));
    }

    /// Helper that formats and returns the output, panicking with a diff on failure.
    fn format_and_check(input: &str, expected: &str) {
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let actual = result.unwrap_or_else(|| input.to_string());
        if actual != expected {
            eprintln!("=== EXPECTED ===\n{expected}\n=== ACTUAL ===\n{actual}\n=== END ===");
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn exact_output_simple_class() {
        let expected = "public class Hello {}\n";
        format_and_check("public class Hello {}\n", expected);
    }

    #[test]
    fn exact_output_class_with_method() {
        let input = "\
public class Hello {
    public static void main(String[] args) {
        System.out.println(\"Hello, world!\");
    }
}
";
        // No blank line after { when source doesn't have one
        format_and_check(input, input);
    }

    #[test]
    fn exact_output_package_imports_class() {
        let expected = "\
package com.example;

import java.util.List;
import java.util.Map;

public class Foo {}
";
        format_and_check(expected, expected);
    }

    #[test]
    fn corrects_bad_indentation() {
        // Badly indented input should be reformatted
        let input = "\
public class Hello {
public void greet() {
System.out.println(\"hi\");
}
}
";
        let expected = "\
public class Hello {
    public void greet() {
        System.out.println(\"hi\");
    }
}
";
        format_and_check(input, expected);
    }

    #[test]
    fn formats_method_invocation() {
        let input = "\
public class Test {
    void test() {
        System.out.println(\"hello\");
        list.add(item);
    }
}
";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("System.out.println(\"hello\");"));
        assert!(output.contains("list.add(item);"));
    }

    #[test]
    fn formats_binary_expression() {
        let input = "\
public class Test {
    void test() {
        int x = a + b * c;
        boolean y = x > 0 && x < 10;
    }
}
";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("a + b * c"));
        assert!(output.contains("x > 0 && x < 10"));
    }

    #[test]
    fn formats_lambda_expression() {
        let input = "\
public class Test {
    void test() {
        list.forEach(x -> System.out.println(x));
        Runnable r = () -> {
            doSomething();
        };
    }
}
";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("x -> System.out.println(x)"));
        assert!(output.contains("() -> {"));
    }

    #[test]
    fn formats_ternary_expression() {
        let input = "\
public class Test {
    void test() {
        int x = a > b ? a : b;
    }
}
";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("a > b ? a : b"));
    }

    #[test]
    fn formats_new_expression() {
        let input = "\
public class Test {
    void test() {
        List<String> list = new ArrayList<>();
        int[] arr = new int[]{1, 2, 3};
    }
}
";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("new ArrayList<>()"));
        assert!(output.contains("new int[]"));
    }

    #[test]
    fn formats_cast_and_instanceof() {
        let input = "\
public class Test {
    void test(Object obj) {
        String s = (String) obj;
        if (obj instanceof String) {
            return;
        }
    }
}
";
        let result = format_text(Path::new("Test.java"), input, &default_config()).unwrap();
        let output = result.unwrap_or_else(|| input.to_string());
        assert!(output.contains("(String) obj"));
        assert!(output.contains("obj instanceof String"));
    }

    #[test]
    fn corrects_missing_spaces() {
        // Missing space before brace
        let input = "\
public class Hello{
    void greet(){
        return;
    }
}
";
        let expected = "\
public class Hello {
    void greet() {
        return;
    }
}
";
        format_and_check(input, expected);
    }
}
