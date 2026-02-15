// Helper test to update all spec files with current formatter output
// Run with: cargo test --test update_specs -- --ignored

use dprint_core::configuration::NewLineKind;
use dprint_plugin_java::configuration::Configuration;
use dprint_plugin_java::format_text::format_text;
use std::fs;
use std::path::Path;

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

fn update_spec_file(path: &std::path::Path) -> Result<bool, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;

    let input_marker = "== input ==";
    let output_marker = "== output ==";

    if !content.contains(input_marker) || !content.contains(output_marker) {
        return Ok(false);
    }

    let input_start = content.find(input_marker).unwrap() + input_marker.len();
    let output_start_marker = content.find(output_marker).unwrap();
    let input = content[input_start..output_start_marker].trim();
    let input_with_nl = format!("{}\n", input);

    // Format the input
    let config = default_config();
    let result = format_text(Path::new("Test.java"), &input_with_nl, &config)?;
    let formatted = result.unwrap_or_else(|| input_with_nl.clone());
    let formatted_trimmed = formatted.trim();

    // Reconstruct the file
    let new_content = format!(
        "{}== input ==\n{}\n== output ==\n{}\n",
        &content[..content.find(input_marker).unwrap()],
        input,
        formatted_trimmed
    );

    if new_content != content {
        fs::write(path, new_content)?;
        return Ok(true);
    }

    Ok(false)
}

#[test]
#[ignore]
fn update_all_specs() {
    let mut updated = 0;
    let mut errors = 0;

    for entry in walkdir::WalkDir::new("tests/specs") {
        if let Ok(entry) = entry
            && entry.path().extension().and_then(|s| s.to_str()) == Some("txt") {
                match update_spec_file(entry.path()) {
                    Ok(true) => {
                        println!("Updated: {}", entry.path().display());
                        updated += 1;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        eprintln!("Error updating {}: {}", entry.path().display(), e);
                        errors += 1;
                    }
                }
            }
    }

    println!("\nUpdated {} spec files with {} errors", updated, errors);
}
