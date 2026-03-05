use std::path::Path;
use std::fs;

fn main() {
    let file_path = std::env::args().nth(1).expect("Usage: test_stability <java-file>");
    let source = fs::read_to_string(&file_path).expect("Failed to read file");

    // Use the library directly
    let config = dprint_plugin_java::configuration::Configuration {
        line_width: 120,
        indent_width: 4,
        use_tabs: false,
        new_line_kind: dprint_core::configuration::NewLineKind::LineFeed,
        format_javadoc: true,
        method_chain_threshold: 80,
        inline_lambdas: true,
    };

    println!("=== Pass 1 ===");
    let pass1 = dprint_plugin_java::format_text(Path::new("test.java"), &source, &config)
        .expect("Pass 1 failed")
        .unwrap_or_else(|| source.clone());

    println!("=== Pass 2 ===");
    let pass2 = dprint_plugin_java::format_text(Path::new("test.java"), &pass1, &config)
        .expect("Pass 2 failed")
        .unwrap_or_else(|| pass1.clone());

    println!("=== Pass 3 ===");
    let pass3 = dprint_plugin_java::format_text(Path::new("test.java"), &pass2, &config)
        .expect("Pass 3 failed")
        .unwrap_or_else(|| pass2.clone());

    if pass1 == pass2 && pass2 == pass3 {
        println!("✓ Formatting is stable!");
    } else {
        println!("✗ Formatting is UNSTABLE!");

        if pass1 != pass2 {
            println!("\n=== Lines that differ pass1 -> pass2 ===");
            let lines1: Vec<&str> = pass1.lines().collect();
            let lines2: Vec<&str> = pass2.lines().collect();
            for (i, (l1, l2)) in lines1.iter().zip(lines2.iter()).enumerate() {
                if l1 != l2 {
                    println!("Line {}: {:?}", i + 1, l1);
                    println!("     -> {:?}", l2);
                }
            }
            if lines1.len() != lines2.len() {
                println!("Line count: {} -> {}", lines1.len(), lines2.len());
            }
        }

        if pass2 != pass3 {
            println!("\n=== Lines that differ pass2 -> pass3 ===");
            let lines2: Vec<&str> = pass2.lines().collect();
            let lines3: Vec<&str> = pass3.lines().collect();
            for (i, (l2, l3)) in lines2.iter().zip(lines3.iter()).enumerate() {
                if l2 != l3 {
                    println!("Line {}: {:?}", i + 1, l2);
                    println!("     -> {:?}", l3);
                }
            }
            if lines2.len() != lines3.len() {
                println!("Line count: {} -> {}", lines2.len(), lines3.len());
            }
        }

        // Write outputs
        fs::write("/tmp/pass1.java", &pass1).ok();
        fs::write("/tmp/pass2.java", &pass2).ok();
        fs::write("/tmp/pass3.java", &pass3).ok();
        println!("\nWrote outputs to /tmp/pass*.java");
    }
}
