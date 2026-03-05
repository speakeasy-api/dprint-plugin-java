use std::path::Path;
use dprint_plugin_java::{configuration::Configuration, format_text};
use dprint_core::configuration::NewLineKind;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <java-file>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    let source = std::fs::read_to_string(file_path).expect("Failed to read file");

    let config = Configuration {
        line_width: 120,
        indent_width: 4,
        use_tabs: false,
        new_line_kind: NewLineKind::LineFeed,
        format_javadoc: true,
        method_chain_threshold: 80,
        inline_lambdas: true,
    };

    println!("Formatting {}...", file_path);

    let pass1 = format_text(Path::new("test.java"), &source, &config)
        .expect("Pass 1 failed")
        .unwrap_or_else(|| source.clone());

    let pass2 = format_text(Path::new("test.java"), &pass1, &config)
        .expect("Pass 2 failed")
        .unwrap_or_else(|| pass1.clone());

    let pass3 = format_text(Path::new("test.java"), &pass2, &config)
        .expect("Pass 3 failed")
        .unwrap_or_else(|| pass2.clone());

    if pass1 == pass2 && pass2 == pass3 {
        println!("✓ Formatting is STABLE");
        std::process::exit(0);
    } else {
        println!("✗ Formatting is UNSTABLE");

        if pass1 != pass2 {
            println!("\n=== Differences between pass1 and pass2 ===");
            print_diff(&pass1, &pass2);
        }

        if pass2 != pass3 {
            println!("\n=== Differences between pass2 and pass3 ===");
            print_diff(&pass2, &pass3);
        }

        std::fs::write("/tmp/pass1.java", &pass1).ok();
        std::fs::write("/tmp/pass2.java", &pass2).ok();
        std::fs::write("/tmp/pass3.java", &pass3).ok();
        println!("\nOutputs written to /tmp/pass*.java");
        std::process::exit(1);
    }
}

fn print_diff(a: &str, b: &str) {
    let lines_a: Vec<&str> = a.lines().collect();
    let lines_b: Vec<&str> = b.lines().collect();

    if lines_a.len() != lines_b.len() {
        println!("Line count differs: {} vs {}", lines_a.len(), lines_b.len());
    }

    let mut diff_count = 0;
    for (i, (line_a, line_b)) in lines_a.iter().zip(lines_b.iter()).enumerate() {
        if line_a != line_b {
            diff_count += 1;
            if diff_count <= 20 { // Only show first 20 diffs
                println!("Line {}:", i + 1);
                println!("  - {:?}", line_a);
                println!("  + {:?}", line_b);
            }
        }
    }

    if diff_count > 20 {
        println!("... and {} more differences", diff_count - 20);
    } else if diff_count == 0 {
        println!("No line differences, but strings differ (whitespace?)");
    } else {
        println!("Total: {} lines differ", diff_count);
    }
}
