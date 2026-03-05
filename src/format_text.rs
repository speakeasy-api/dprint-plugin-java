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
    fn preserves_blank_after_line_comment_before_javadoc() {
        let input = "\
public class Foo {

    void bar() {}

    // Section header

    /**
     * Does stuff.
     */
    void baz() {}
}
";
        format_and_check(input, input);
    }

    #[test]
    fn sorts_static_imports_alphabetically() {
        let input = "\
package org.openapis.review.openapi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertArrayEquals;

public class Tag2Tests {
    void test() {}
}
";
        let config = default_config();
        let result = format_text(Path::new("Test.java"), input, &config)
            .unwrap()
            .unwrap_or_else(|| input.to_string());
        let lines: Vec<&str> = result.lines().collect();
        let import_lines: Vec<&&str> = lines
            .iter()
            .filter(|l| l.starts_with("import static"))
            .collect();
        assert_eq!(import_lines.len(), 2);
        assert!(
            import_lines[0].contains("assertArrayEquals"),
            "assertArrayEquals should come first, got: {:?}",
            import_lines
        );
        assert!(
            import_lines[1].contains("assertEquals"),
            "assertEquals should come second, got: {:?}",
            import_lines
        );
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

    /// Assert that formatting is idempotent: format(format(input)) == format(input).
    /// Formats up to 3 times and checks each pass produces the same output.
    fn assert_idempotent(input: &str) {
        let config = default_config();
        let pass1 = format_text(Path::new("Test.java"), input, &config)
            .unwrap()
            .unwrap_or_else(|| input.to_string());
        let pass2 = format_text(Path::new("Test.java"), &pass1, &config)
            .unwrap()
            .unwrap_or_else(|| pass1.clone());
        if pass1 != pass2 {
            // Print a unified diff-style comparison
            let lines1: Vec<&str> = pass1.lines().collect();
            let lines2: Vec<&str> = pass2.lines().collect();
            let mut diff_lines = Vec::new();
            for (i, (a, b)) in lines1.iter().zip(lines2.iter()).enumerate() {
                if a != b {
                    diff_lines.push(format!("Line {}: ", i + 1));
                    diff_lines.push(format!("  pass1: {a:?}"));
                    diff_lines.push(format!("  pass2: {b:?}"));
                }
            }
            if lines1.len() != lines2.len() {
                diff_lines.push(format!(
                    "Line count: pass1={} pass2={}",
                    lines1.len(),
                    lines2.len()
                ));
            }
            panic!(
                "Formatting is not idempotent!\nDiffs:\n{}",
                diff_lines.join("\n")
            );
        }
        let pass3 = format_text(Path::new("Test.java"), &pass2, &config)
            .unwrap()
            .unwrap_or_else(|| pass2.clone());
        assert_eq!(
            pass2, pass3,
            "Formatting flip-flops on pass 3!\n=== Pass 2 ===\n{pass2}\n=== Pass 3 ===\n{pass3}"
        );
    }

    // ---- Idempotency tests for SDK usage snippet patterns ----

    #[test]
    fn idempotent_builder_chain_near_threshold() {
        // Builder chain where dot positions are near the 80-col method_chain_threshold
        assert_idempotent(
            "\
package hello.world;

public class Application {
    public static void main(String[] args) {
        SDK sdk = SDK.builder()
                .deprecatedQueryParam1(\"some example query param\")
                .deprecatedQueryParam2(\"some example query param\")
                .build();
    }
}
",
        );
    }

    #[test]
    fn idempotent_nested_builder_chain() {
        // Nested builder calls — common in SDK usage snippets
        assert_idempotent(
            "\
package hello.world;

public class Application {
    public static void main(String[] args) {
        PostFileRequest req = PostFileRequest.builder()
                .upload(File.builder()
                        .fileName(\"example.file\")
                        .content(Blob.from(Paths.get(\"example.file\")))
                        .build())
                .build();

        PostFileResponse res = sdk.postFile().request(req).call();
    }
}
",
        );
    }

    #[test]
    fn idempotent_deeply_nested_builder() {
        // Deeply nested builder pattern from actual SDK retry config
        assert_idempotent(
            "\
package hello.world;

import java.util.concurrent.TimeUnit;

public class Application {
    public static void main(String[] args) throws Exception {
        PostTest2Response res = sdk.testGroup().tag2().postTest()
                .retryConfig(RetryConfig.builder()
                        .backoff(BackoffStrategy.builder()
                                .initialInterval(1L, TimeUnit.MILLISECONDS)
                                .maxInterval(50L, TimeUnit.MILLISECONDS)
                                .maxElapsedTime(1000L, TimeUnit.MILLISECONDS)
                                .baseFactor(1.1)
                                .jitterFactor(0.15)
                                .retryConnectError(false)
                                .build())
                        .build())
                .call();
    }
}
",
        );
    }

    #[test]
    fn idempotent_chain_at_line_width_boundary() {
        // Chain that is right at line_width=120 boundary
        assert_idempotent(
            "\
public class Test {
    void test() {
        SomeVeryLongTypeName result = someObject.methodOne().methodTwo().methodThree().methodFour(\"argument\");
    }
}
",
        );
    }

    #[test]
    fn idempotent_binary_expr_at_boundary() {
        // Binary expression right at line_width boundary
        assert_idempotent(
            "\
public class Test {
    void test() {
        boolean result = someConditionVariable && anotherConditionVariable || thirdConditionVariable && fourthCondition;
    }
}
",
        );
    }

    #[test]
    fn idempotent_ternary_at_boundary() {
        // Ternary expression near line_width boundary
        assert_idempotent(
            "\
public class Test {
    void test() {
        String value = someCondition ? someReallyLongResultExpression : anotherReallyLongAlternativeExpression;
    }
}
",
        );
    }

    #[test]
    fn idempotent_method_chain_with_lambda() {
        // Method chain with lambda — common in SDK snippets
        assert_idempotent(
            "\
public class Test {
    void test() {
        CompletableFuture<Response> future = client.sendAsync(request, response -> {
            return response.body();
        });
    }
}
",
        );
    }

    #[test]
    fn idempotent_long_throws_clause() {
        // Method with long throws clause — from SDK generated code
        assert_idempotent(
            "\
public class Application {
    public static void main(String[] args) throws BadRequestResponseException, Error, Test2ResponseException, Exception {
        SDK sdk = SDK.builder().build();
    }
}
",
        );
    }

    #[test]
    fn idempotent_map_of_entries() {
        // Map.ofEntries with nested builders — common in SDK test data
        assert_idempotent(
            "\
public class Test {
    void test() {
        ExhaustiveObject obj = ExhaustiveObject.builder()
                .map(Map.ofEntries(
                        Map.entry(\"key\", SimpleObject.builder()
                                .str(\"example\")
                                .build())))
                .arr(List.of(
                        SimpleObject.builder()
                                .str(\"example\")
                                .build(),
                        SimpleObject.builder()
                                .str(\"example\")
                                .build()))
                .build();
    }
}
",
        );
    }

    #[test]
    fn idempotent_chain_with_type_args() {
        // Generic method invocation chain
        assert_idempotent(
            "\
public class Test {
    void test() {
        AsyncRequestOperation<BinaryAndStringUploadRequest, BinaryAndStringUploadResponse> op =
                client.<BinaryAndStringUploadRequest, BinaryAndStringUploadResponse>createOperation()
                        .withTimeout(Duration.ofSeconds(30))
                        .build();
    }
}
",
        );
    }

    #[test]
    fn idempotent_unformatted_builder_chain() {
        // Unformatted builder chain (as generator might produce)
        assert_idempotent(
            "\
package hello.world;

public class Application {
    public static void main(String[] args) {
        SDK sdk = SDK.builder().deprecatedQueryParam1(\"some example query param\").deprecatedQueryParam2(\"some example query param\").build();
    }
}
",
        );
    }

    #[test]
    fn idempotent_unformatted_nested_builder() {
        // Completely unformatted nested builder (single line, as generator might output)
        assert_idempotent(
            "\
package hello.world;

public class Application {
    public static void main(String[] args) {
        PostFileRequest req = PostFileRequest.builder().upload(File.builder().fileName(\"example.file\").content(Blob.from(Paths.get(\"example.file\"))).build()).build();
    }
}
",
        );
    }

    #[test]
    fn idempotent_if_condition_binary_expr() {
        // Binary expression inside if condition near boundary (suffix_width = 3 for `) {`)
        assert_idempotent(
            "\
public class Test {
    void test() {
        if (someReallyLongVariable != null && anotherReallyLongVariable != null && thirdVariable.isPresent()) {
            doSomething();
        }
    }
}
",
        );
    }

    #[test]
    fn idempotent_return_with_chain() {
        // Return statement with method chain
        assert_idempotent(
            "\
public class Test {
    Response test() {
        return client.target(baseUrl).path(\"/api/v1/resource\").request().accept(MediaType.APPLICATION_JSON).get();
    }
}
",
        );
    }

    #[test]
    fn idempotent_assignment_with_long_rhs_chain() {
        // Assignment where RHS is a long chain that might wrap at '='
        assert_idempotent(
            "\
public class Test {
    void test() {
        SomeVeryLongResultType result = SomeFactory.getInstance().createBuilder().withParam1(\"value1\").withParam2(\"value2\").build();
    }
}
",
        );
    }

    /// Check if the given input is already a fixed point (format(input) == input).
    /// If not, print the diff and panic.
    fn assert_already_formatted(input: &str) {
        let config = default_config();
        let result = format_text(Path::new("Test.java"), input, &config).unwrap();
        if let Some(formatted) = result {
            panic!(
                "Input is NOT a fixed point of the formatter!\n=== INPUT ===\n{input}\n=== FORMATTED ===\n{formatted}"
            );
        }
    }

    #[test]
    fn real_sdk_usage_snippet_before_is_fixed_point() {
        // Test if the "before" version from the SDK is already correctly formatted
        assert_already_formatted(
            r#"package hello.world;

import java.lang.Exception;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Map;
import org.openapis.review.openapi.SDK;
import org.openapis.review.openapi.models.errors.*;
import org.openapis.review.openapi.models.errors.Error;
import org.openapis.review.openapi.models.operations.PostTest2Response;
import org.openapis.review.openapi.models.shared.*;

public class Application {

    public static void main(String[] args)
            throws BadRequestResponseException, Error, Test2ResponseException, Exception {

        SDK sdk = SDK.builder()
                .deprecatedQueryParam1("some example query param")
                .deprecatedQueryParam2("some example query param")
                .build();

        PostTest2Response res = sdk.testGroup()
                .tag2()
                .postTest()
                .test2Request(Test2Request.builder()
                        .obj(ExhaustiveObject.builder()
                                .str("example")
                                .bool(true)
                                .integer(999999L)
                                .int32(1)
                                .num(1.1)
                                .float32(8499.3f)
                                .date(LocalDate.parse("2020-01-01"))
                                .dateTime(OffsetDateTime.parse("2020-01-01T00:00:00Z"))
                                .anything("<value>")
                                .int32Enum(Int32Enum.SIXTY_NINE)
                                .bigint(new BigInteger("702830"))
                                .decimalStr(new BigDecimal("3858.6"))
                                .obj(SimpleObject.builder().str("example").build())
                                .map(Map.ofEntries(
                                        Map.entry(
                                                "key",
                                                SimpleObject.builder()
                                                        .str("example")
                                                        .build())))
                                .arr(List.of(
                                        SimpleObject.builder().str("example").build(),
                                        SimpleObject.builder().str("example").build()))
                                .any(Any.of(SimpleObject.builder().str("example").build()))
                                .nullableStringEnum(NullableStringEnum.SECOND)
                                .icon(Icon.TICK)
                                .boolOpt(true)
                                .intOptNull(999999L)
                                .numOptNull(1.1)
                                .intEnum(IntEnum.Third)
                                .nullableIntEnum(NullableIntEnum.Third)
                                .color(Color.GREEN)
                                .heroWidth(HeroWidth.FOUR_HUNDRED_AND_EIGHTY)
                                .build())
                        .type(Type.SuperType1)
                        .build())
                .call();

        if (res.body().isPresent()) {
            // handle response
        }
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_real_sdk_usage_snippet_before() {
        // Actual SDK usage snippet from openapi-generation (before version)
        assert_idempotent(
            r#"package hello.world;

import java.lang.Exception;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Map;
import org.openapis.review.openapi.SDK;
import org.openapis.review.openapi.models.errors.*;
import org.openapis.review.openapi.models.errors.Error;
import org.openapis.review.openapi.models.operations.PostTest2Response;
import org.openapis.review.openapi.models.shared.*;

public class Application {

    public static void main(String[] args)
            throws BadRequestResponseException, Error, Test2ResponseException, Exception {

        SDK sdk = SDK.builder()
                .deprecatedQueryParam1("some example query param")
                .deprecatedQueryParam2("some example query param")
                .build();

        PostTest2Response res = sdk.testGroup()
                .tag2()
                .postTest()
                .test2Request(Test2Request.builder()
                        .obj(ExhaustiveObject.builder()
                                .str("example")
                                .bool(true)
                                .integer(999999L)
                                .int32(1)
                                .num(1.1)
                                .float32(8499.3f)
                                .date(LocalDate.parse("2020-01-01"))
                                .dateTime(OffsetDateTime.parse("2020-01-01T00:00:00Z"))
                                .anything("<value>")
                                .int32Enum(Int32Enum.SIXTY_NINE)
                                .bigint(new BigInteger("702830"))
                                .decimalStr(new BigDecimal("3858.6"))
                                .obj(SimpleObject.builder().str("example").build())
                                .map(Map.ofEntries(
                                        Map.entry(
                                                "key",
                                                SimpleObject.builder()
                                                        .str("example")
                                                        .build())))
                                .arr(List.of(
                                        SimpleObject.builder().str("example").build(),
                                        SimpleObject.builder().str("example").build()))
                                .any(Any.of(SimpleObject.builder().str("example").build()))
                                .nullableStringEnum(NullableStringEnum.SECOND)
                                .icon(Icon.TICK)
                                .boolOpt(true)
                                .intOptNull(999999L)
                                .numOptNull(1.1)
                                .intEnum(IntEnum.Third)
                                .nullableIntEnum(NullableIntEnum.Third)
                                .color(Color.GREEN)
                                .heroWidth(HeroWidth.FOUR_HUNDRED_AND_EIGHTY)
                                .build())
                        .type(Type.SuperType1)
                        .build())
                .call();

        if (res.body().isPresent()) {
            // handle response
        }
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_raw_template_output() {
        // Simulate what the Go template engine might produce (unformatted, single-line chains)
        // with explicit imports (the fix-java-wildcard-doc-imports version)
        assert_idempotent(
            r#"package hello.world;

import java.lang.Exception;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Map;
import org.openapis.review.openapi.SDK;
import org.openapis.review.openapi.models.errors.BadRequestResponseException;
import org.openapis.review.openapi.models.errors.Error;
import org.openapis.review.openapi.models.errors.Test2ResponseException;
import org.openapis.review.openapi.models.operations.PostTest2Response;
import org.openapis.review.openapi.models.shared.Any;
import org.openapis.review.openapi.models.shared.Color;
import org.openapis.review.openapi.models.shared.ExhaustiveObject;
import org.openapis.review.openapi.models.shared.HeroWidth;
import org.openapis.review.openapi.models.shared.Icon;
import org.openapis.review.openapi.models.shared.Int32Enum;
import org.openapis.review.openapi.models.shared.IntEnum;
import org.openapis.review.openapi.models.shared.NullableIntEnum;
import org.openapis.review.openapi.models.shared.NullableStringEnum;
import org.openapis.review.openapi.models.shared.SimpleObject;
import org.openapis.review.openapi.models.shared.Test2Request;
import org.openapis.review.openapi.models.shared.Type;

public class Application {
    public static void main(String[] args) throws BadRequestResponseException, Error, Test2ResponseException, Exception {
        SDK sdk = SDK.builder().deprecatedQueryParam1("some example query param").deprecatedQueryParam2("some example query param").build();
        PostTest2Response res = sdk.testGroup().tag2().postTest().test2Request(Test2Request.builder().obj(ExhaustiveObject.builder().str("example").bool(true).integer(999999L).int32(1).num(1.1).float32(8499.3f).date(LocalDate.parse("2020-01-01")).dateTime(OffsetDateTime.parse("2020-01-01T00:00:00Z")).anything("<value>").int32Enum(Int32Enum.SIXTY_NINE).bigint(new BigInteger("702830")).decimalStr(new BigDecimal("3858.6")).obj(SimpleObject.builder().str("example").build()).map(Map.ofEntries(Map.entry("key", SimpleObject.builder().str("example").build()))).arr(List.of(SimpleObject.builder().str("example").build(), SimpleObject.builder().str("example").build())).any(Any.of(SimpleObject.builder().str("example").build())).nullableStringEnum(NullableStringEnum.SECOND).icon(Icon.TICK).boolOpt(true).intOptNull(999999L).numOptNull(1.1).intEnum(IntEnum.Third).nullableIntEnum(NullableIntEnum.Third).color(Color.GREEN).heroWidth(HeroWidth.FOUR_HUNDRED_AND_EIGHTY).build()).type(Type.SuperType1).build()).call();
        if (res.body().isPresent()) {
            // handle response
        }
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_any_of_wrapping_minimal() {
        // Minimal repro for the flip-flop bug: .any(Any.of(SimpleObject.builder()...))
        // would wrap on pass 1 but unwrap on pass 2 because estimate_prefix_width
        // walked past argument_list boundaries, giving source-position-dependent results.
        assert_idempotent(
            r#"public class Test {
    void test() {
        ExhaustiveObject obj = ExhaustiveObject.builder().any(Any.of(SimpleObject.builder().str("example").build())).build();
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_method_arg_chain_wrapping() {
        // Method call argument that is itself a chain — tests wrapping of chain inside arg list
        assert_idempotent(
            r#"public class Test {
    void test() {
        Foo foo = Foo.builder()
                .bar(Bar.of(Baz.builder().field1("value1").field2("value2").field3("value3").build()))
                .build();
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_deeply_nested_at_continuation_indent() {
        // Chain at deep continuation indent where argument contains another chain
        assert_idempotent(
            r#"public class Test {
    void test() {
        Result res = client.service().operation().request(Request.builder().obj(Obj.builder().str("example").bool(true).integer(999999L).int32(1).num(1.1).float32(8499.3f).obj(SimpleObject.builder().str("example").build()).any(Any.of(SimpleObject.builder().str("example").build())).build()).type(Type.SuperType1).build()).call();
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_raw_template_with_wildcard_imports() {
        // Same but with wildcard imports (changes throws clause width calculation)
        assert_idempotent(
            r#"package hello.world;

import java.lang.Exception;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Map;
import org.openapis.review.openapi.SDK;
import org.openapis.review.openapi.models.errors.*;
import org.openapis.review.openapi.models.errors.Error;
import org.openapis.review.openapi.models.operations.PostTest2Response;
import org.openapis.review.openapi.models.shared.*;

public class Application {
    public static void main(String[] args) throws BadRequestResponseException, Error, Test2ResponseException, Exception {
        SDK sdk = SDK.builder().deprecatedQueryParam1("some example query param").deprecatedQueryParam2("some example query param").build();
        PostTest2Response res = sdk.testGroup().tag2().postTest().test2Request(Test2Request.builder().obj(ExhaustiveObject.builder().str("example").bool(true).integer(999999L).int32(1).num(1.1).float32(8499.3f).date(LocalDate.parse("2020-01-01")).dateTime(OffsetDateTime.parse("2020-01-01T00:00:00Z")).anything("<value>").int32Enum(Int32Enum.SIXTY_NINE).bigint(new BigInteger("702830")).decimalStr(new BigDecimal("3858.6")).obj(SimpleObject.builder().str("example").build()).map(Map.ofEntries(Map.entry("key", SimpleObject.builder().str("example").build()))).arr(List.of(SimpleObject.builder().str("example").build(), SimpleObject.builder().str("example").build())).any(Any.of(SimpleObject.builder().str("example").build())).nullableStringEnum(NullableStringEnum.SECOND).icon(Icon.TICK).boolOpt(true).intOptNull(999999L).numOptNull(1.1).intEnum(IntEnum.Third).nullableIntEnum(NullableIntEnum.Third).color(Color.GREEN).heroWidth(HeroWidth.FOUR_HUNDRED_AND_EIGHTY).build()).type(Type.SuperType1).build()).call();
        if (res.body().isPresent()) {
            // handle response
        }
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_real_sdk_usage_snippet_after() {
        // Actual SDK usage snippet from openapi-generation (after version)
        assert_idempotent(
            r#"package hello.world;

import java.lang.Exception;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Map;
import org.openapis.review.openapi.SDK;
import org.openapis.review.openapi.models.errors.*;
import org.openapis.review.openapi.models.errors.Error;
import org.openapis.review.openapi.models.operations.PostTest2Response;
import org.openapis.review.openapi.models.shared.*;

public class Application {

    public static void main(String[] args) throws BadRequestResponseException, Error, Test2ResponseException, Exception {

        SDK sdk = SDK.builder()
                .deprecatedQueryParam1("some example query param")
                .deprecatedQueryParam2("some example query param")
            .build();

        PostTest2Response res = sdk.testGroup().tag2().postTest()
                .test2Request(Test2Request.builder()
                    .obj(ExhaustiveObject.builder()
                        .str("example")
                        .bool(true)
                        .integer(999999L)
                        .int32(1)
                        .num(1.1)
                        .float32(8499.3f)
                        .date(LocalDate.parse("2020-01-01"))
                        .dateTime(OffsetDateTime.parse("2020-01-01T00:00:00Z"))
                        .anything("<value>")
                        .int32Enum(Int32Enum.SIXTY_NINE)
                        .bigint(new BigInteger("702830"))
                        .decimalStr(new BigDecimal("3858.6"))
                        .obj(SimpleObject.builder()
                            .str("example")
                            .build())
                        .map(Map.ofEntries(
                            Map.entry("key", SimpleObject.builder()
                                .str("example")
                                .build())))
                        .arr(List.of(
                            SimpleObject.builder()
                                .str("example")
                                .build(),
                            SimpleObject.builder()
                                .str("example")
                                .build()))
                        .any(Any.of(SimpleObject.builder()
                            .str("example")
                            .build()))
                        .nullableStringEnum(NullableStringEnum.SECOND)
                        .icon(Icon.TICK)
                        .boolOpt(true)
                        .intOptNull(999999L)
                        .numOptNull(1.1)
                        .intEnum(IntEnum.Third)
                        .nullableIntEnum(NullableIntEnum.Third)
                        .color(Color.GREEN)
                        .heroWidth(HeroWidth.FOUR_HUNDRED_AND_EIGHTY)
                        .build())
                    .type(Type.SuperType1)
                    .build())
                .call();

        if (res.body().isPresent()) {
            // handle response
        }
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_wasm_crash_example() {
        // This exact file crashes the WASM build with OOB memory access in flatten_chain
        assert_idempotent(
            r#"package hello.world;

import java.lang.Exception;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Map;
import org.openapis.review.openapi.SDK;
import org.openapis.review.openapi.models.errors.*;
import org.openapis.review.openapi.models.errors.Error;
import org.openapis.review.openapi.models.operations.PostTest2Response;
import org.openapis.review.openapi.models.shared.*;

public class Application {

    public static void main(String[] args) throws BadRequestResponseException, Error, Test2ResponseException, Exception {

        SDK sdk = SDK.builder()
                .deprecatedQueryParam1("some example query param")
                .deprecatedQueryParam2("some example query param")
            .build();

        PostTest2Response res = sdk.testGroup().tag2().postTest()
                .test2Request(Test2Request.builder()
                    .obj(ExhaustiveObject.builder()
                        .str("example")
                        .bool(true)
                        .integer(999999L)
                        .int32(1)
                        .num(1.1)
                        .float32(8499.3f)
                        .date(LocalDate.parse("2020-01-01"))
                        .dateTime(OffsetDateTime.parse("2020-01-01T00:00:00Z"))
                        .anything("<value>")
                        .int32Enum(Int32Enum.SIXTY_NINE)
                        .bigint(new BigInteger("702830"))
                        .decimalStr(new BigDecimal("3858.6"))
                        .obj(SimpleObject.builder()
                            .str("example")
                            .build())
                        .map(Map.ofEntries(
                            Map.entry("key", SimpleObject.builder()
                                .str("example")
                                .build())))
                        .arr(List.of(
                            SimpleObject.builder()
                                .str("example")
                                .build(),
                            SimpleObject.builder()
                                .str("example")
                                .build()))
                        .any(Any.of(SimpleObject.builder()
                            .str("example")
                            .build()))
                        .nullableStringEnum(NullableStringEnum.SECOND)
                        .icon(Icon.TICK)
                        .boolOpt(true)
                        .intOptNull(999999L)
                        .numOptNull(1.1)
                        .intEnum(IntEnum.Third)
                        .nullableIntEnum(NullableIntEnum.Third)
                        .color(Color.GREEN)
                        .heroWidth(HeroWidth.FOUR_HUNDRED_AND_EIGHTY)
                        .build())
                    .type(Type.SuperType1)
                    .build())
                .call();

        if (res.body().isPresent()) {
            // handle response
        }
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_tag2tests_real_file() {
        // Exact content from a generated Tag2Tests.java — must be a fixed point.
        assert_idempotent(
            r#"/*
 * Code generated by Speakeasy (https://speakeasy.com). DO NOT EDIT.
 */
package org.openapis.review.openapi;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;

import java.lang.Exception;
import java.math.BigDecimal;
import java.math.BigInteger;
import java.nio.charset.StandardCharsets;
import java.time.LocalDate;
import java.time.OffsetDateTime;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.openapis.review.openapi.models.operations.PostTest2Response;
import org.openapis.review.openapi.models.shared.Any;
import org.openapis.review.openapi.models.shared.Color;
import org.openapis.review.openapi.models.shared.ExhaustiveObject;
import org.openapis.review.openapi.models.shared.HeroWidth;
import org.openapis.review.openapi.models.shared.Icon;
import org.openapis.review.openapi.models.shared.Int32Enum;
import org.openapis.review.openapi.models.shared.IntEnum;
import org.openapis.review.openapi.models.shared.NullableIntEnum;
import org.openapis.review.openapi.models.shared.NullableStringEnum;
import org.openapis.review.openapi.models.shared.SimpleObject;
import org.openapis.review.openapi.models.shared.Test2Request;
import org.openapis.review.openapi.models.shared.Type;
import org.openapis.review.openapi.utils.Utils;

public class Tag2Tests {

    @Test
    public void testTag2_PostTest2() throws Exception {

        var testHttpClient = Utils.createTestHTTPClient("postTest2");
        SDK sdk = SDK.builder().client(testHttpClient).build();

        PostTest2Response res = sdk.testGroup()
                .tag2()
                .postTest()
                .serverURL(Utils.environmentVariable("TEST_SERVER_URL", "http://localhost:18080"))
                .deprecatedQueryParam1("some example query param")
                .deprecatedQueryParam2("some example query param")
                .test2Request(Test2Request.builder()
                        .obj(ExhaustiveObject.builder()
                                .str("example")
                                .bool(true)
                                .integer(999999L)
                                .int32(1)
                                .num(1.1)
                                .float32(8499.3f)
                                .date(LocalDate.parse("2020-01-01"))
                                .dateTime(OffsetDateTime.parse("2020-01-01T00:00:00Z"))
                                .anything("<value>")
                                .int32Enum(Int32Enum.SIXTY_NINE)
                                .bigint(new BigInteger("702830"))
                                .decimalStr(new BigDecimal("3858.6"))
                                .obj(SimpleObject.builder().str("example").build())
                                .map(Map.ofEntries(
                                        Map.entry(
                                                "key",
                                                SimpleObject.builder()
                                                        .str("example")
                                                        .build())))
                                .arr(List.of(
                                        SimpleObject.builder().str("example").build(),
                                        SimpleObject.builder().str("example").build()))
                                .any(Any.of(SimpleObject.builder().str("example").build()))
                                .nullableStringEnum(NullableStringEnum.SECOND)
                                .icon(Icon.TICK)
                                .boolOpt(true)
                                .intOptNull(999999L)
                                .numOptNull(1.1)
                                .intEnum(IntEnum.Third)
                                .nullableIntEnum(NullableIntEnum.Third)
                                .color(Color.GREEN)
                                .heroWidth(HeroWidth.FOUR_HUNDRED_AND_EIGHTY)
                                .build())
                        .type(Type.SuperType1)
                        .build())
                .call();
        assertEquals(200, res.statusCode());
        assertArrayEquals(
                "0x20D83Acf0f".getBytes(StandardCharsets.UTF_8), res.body().orElse(null));
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_binary_expr_in_return_with_long_chain() {
        // Binary expressions in return statements should wrap stably.
        // The issue was that wrapping decisions depended on source column positions,
        // which change between passes, causing oscillation.
        assert_idempotent(
            r#"public class Test {
    void test() {
        return httpClientConfiguration.getRedactedHeaders() != null
                && !httpClientConfiguration.getRedactedHeaders().isEmpty();
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_nested_builder_with_binary_condition() {
        // Complex nested builder pattern with binary expressions in conditions.
        // This pattern was failing in the Jahia codebase.
        assert_idempotent(
            r#"public class Test {
    void test() {
        Result res = client
                .request(Request.builder()
                    .obj(Obj.builder()
                        .field1("value1")
                        .field2("value2")
                        .build())
                    .build())
                .call();
        
        if (res != null && res.body().isPresent() && res.statusCode() == 200) {
            handleSuccess(res);
        }
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_ternary_in_assignment_chain() {
        // Ternary expressions can also cause instability if wrapping decisions
        // depend on source positions rather than formatted positions.
        assert_idempotent(
            r#"public class Test {
    void test() {
        String msg = httpClient.getConfig() != null ? httpClient.getConfig().getStatusMessage() : "Unknown";
    }
}
"#,
        );
    }

    #[test]
    fn idempotent_long_binary_chain_in_condition() {
        // Long chains of binary operators that need wrapping,
        // especially in if/while/for conditions where the row-based
        // estimate_prefix_width logic could oscillate.
        assert_idempotent(
            r#"public class Test {
    void test() {
        if (config != null && config.isValid() && config.getTimeout() > 0 && !isShutdown()) {
            processRequest();
        }
    }
}
"#,
        );
    }
}
