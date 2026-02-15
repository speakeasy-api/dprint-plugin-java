use std::path::Path;

use anyhow::Result;
use dprint_core::configuration::resolve_new_line_kind;
use dprint_core::formatting::PrintOptions;

use crate::configuration::Configuration;
use crate::generation::generate;

/// Format a Java source file. Returns `Ok(None)` if no changes were made.
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
        .map_err(|e| anyhow::anyhow!("Failed to load Java grammar: {}", e))?;

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
    fn pass_through_simple_class() {
        let input = r#"public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello, world!");
    }
}
"#;
        let result = format_text(Path::new("Hello.java"), input, &default_config()).unwrap();
        // Pass-through: no changes expected
        assert!(result.is_none());
    }

    #[test]
    fn handles_parse_error_gracefully() {
        let input = "public class { broken syntax";
        let result = format_text(Path::new("Bad.java"), input, &default_config()).unwrap();
        // Should return None (unchanged) for parse errors
        assert!(result.is_none());
    }
}
