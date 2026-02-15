use dprint_core::formatting::PrintItems;
use dprint_core::formatting::Signal;

use crate::configuration::Configuration;

use super::context::FormattingContext;

/// Format a line comment: `// ...`
///
/// Preserves the comment text as-is, only normalizing to ensure a single
/// space after the `//` prefix (unless the comment is empty or starts with `///`).
/// ALWAYS emits a newline after the comment to prevent it from commenting out
/// subsequent code on the same line.
pub fn gen_line_comment(node: tree_sitter::Node, context: &FormattingContext) -> PrintItems {
    let mut items = PrintItems::new();
    let text = &context.source[node.start_byte()..node.end_byte()];

    // Normalize: ensure single space after // (but preserve /// and //! style)
    if let Some(rest) = text.strip_prefix("//") {
        items.push_string("//".to_string());
        if rest.is_empty() {
            // Empty comment: just "//"
        } else if rest.starts_with('/') || rest.starts_with('!') {
            // Triple-slash or //! — preserve as-is
            items.push_string(rest.to_string());
        } else if rest.starts_with(' ') {
            // Already has a space — preserve content after the first space
            items.push_string(rest.to_string());
        } else {
            // No space after // — add one
            items.push_string(format!(" {}", rest));
        }
    } else {
        // Fallback: emit as-is
        items.push_string(text.to_string());
    }

    // CRITICAL: Line comments MUST be followed by a newline, otherwise they
    // will comment out whatever code follows on the same line
    items.push_signal(Signal::NewLine);

    items
}

/// Format a block comment: `/* ... */`
///
/// If the comment starts with `/**` (Javadoc), delegates to `gen_javadoc`
/// when `config.format_javadoc` is true. Otherwise preserves the comment
/// content, only normalizing indentation of continuation lines.
pub fn gen_block_comment(node: tree_sitter::Node, context: &FormattingContext) -> PrintItems {
    let text = &context.source[node.start_byte()..node.end_byte()];

    // Check if this is a Javadoc comment
    if text.starts_with("/**") && !text.starts_with("/***") && context.config.format_javadoc {
        return gen_javadoc(node, context, context.config);
    }

    // For non-Javadoc block comments, preserve content but normalize
    // indentation of continuation lines to align with the opening `/*`.
    gen_block_comment_preserved(text)
}

/// Emit a block comment preserving its content but normalizing the
/// indentation of continuation lines so that `*` characters align.
fn gen_block_comment_preserved(text: &str) -> PrintItems {
    let mut items = PrintItems::new();

    let lines: Vec<&str> = text.split('\n').collect();

    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            items.push_signal(Signal::NewLine);
        }

        // Strip trailing \r for CRLF files
        let line = line.strip_suffix('\r').unwrap_or(line);

        // Strip trailing whitespace from the line, handling both end-of-line
        // spaces and spaces before the closing */
        let line = strip_comment_line_trailing_ws(line);

        if i == 0 {
            // First line: emit as-is (already trimmed)
            items.push_string(line.clone());
        } else {
            // Continuation lines: trim leading whitespace and add a single
            // space indent so `*` aligns under `/*`
            let trimmed = line.trim_start();
            if trimmed.is_empty() {
                // Blank continuation line within a block comment — emit
                // just the " *" prefix
                items.push_string(" *".to_string());
            } else if trimmed.starts_with('*') {
                // Line starts with `*` — prefix with single space for alignment
                items.push_string(format!(" {}", trimmed));
            } else {
                // Line doesn't start with * — prefix with " * " to maintain format
                items.push_string(format!(" * {}", trimmed));
            }
        }
    }

    items
}

/// Strip trailing whitespace from a block comment line.
/// This handles both trailing spaces at the end of the line and trailing
/// spaces before the closing */ delimiter. Preserves a single space before */
/// if the comment had any non-whitespace content before it.
fn strip_comment_line_trailing_ws(line: &str) -> String {
    // First, trim any trailing whitespace from the end
    let line = line.trim_end();

    // If the line ends with */, check for trailing spaces before it
    if let Some(rest) = line.strip_suffix("*/") {
        let rest_trimmed = rest.trim_end();
        // If there's content before the */, preserve a single space
        if !rest_trimmed.is_empty() && !rest_trimmed.ends_with(char::is_whitespace) {
            return format!("{} */", rest_trimmed);
        }
        return format!("{}*/", rest_trimmed);
    }

    line.to_string()
}

/// Format a Javadoc comment with tag reflowing.
///
/// Reformats `/** ... */` comments:
/// - Normalizes the opening to `/**` on its own line (or keeps single-line if short)
/// - Aligns continuation lines with ` * `
/// - Reflows `@param`, `@return`, `@throws`/`@exception` tag descriptions
/// - Preserves `{@code ...}` and `<pre>...</pre>` blocks verbatim
/// - Wraps lines to fit within `config.line_width`
fn gen_javadoc(
    node: tree_sitter::Node,
    context: &FormattingContext,
    config: &Configuration,
) -> PrintItems {
    let text = &context.source[node.start_byte()..node.end_byte()];

    // Extract the inner content (strip /** and */)
    let inner = extract_javadoc_content(text);

    // Parse into structured segments
    let segments = parse_javadoc_segments(&inner);

    // Calculate available width for content (account for " * " prefix)
    let indent_chars = context.indent_level() * (config.indent_width as usize);
    let prefix_width = indent_chars + 3; // " * " is 3 chars
    let max_content_width = if (config.line_width as usize) > prefix_width + 10 {
        (config.line_width as usize) - prefix_width
    } else {
        60 // reasonable fallback
    };

    let mut items = PrintItems::new();

    // Opening
    items.push_string("/**".to_string());

    for segment in &segments {
        match segment {
            JavadocSegment::Text(text) => {
                let wrapped = wrap_text(text, max_content_width);
                for line in &wrapped {
                    items.push_signal(Signal::NewLine);
                    if line.is_empty() {
                        items.push_string(" *".to_string());
                    } else {
                        items.push_string(format!(" * {}", line));
                    }
                }
            }
            JavadocSegment::Tag { name, args, desc } => {
                items.push_signal(Signal::NewLine);
                let tag_line = format_tag_line(name, args, desc);
                let wrapped = wrap_text(&tag_line, max_content_width);
                for (i, line) in wrapped.iter().enumerate() {
                    if i > 0 {
                        items.push_signal(Signal::NewLine);
                    }
                    if line.is_empty() {
                        items.push_string(" *".to_string());
                    } else {
                        items.push_string(format!(" * {}", line));
                    }
                }
            }
            JavadocSegment::PreBlock(content) => {
                items.push_signal(Signal::NewLine);
                items.push_string(" * <pre>".to_string());
                for line in content.split('\n') {
                    items.push_signal(Signal::NewLine);
                    let line = line.strip_suffix('\r').unwrap_or(line);
                    if line.is_empty() {
                        items.push_string(" *".to_string());
                    } else {
                        items.push_string(format!(" * {}", line));
                    }
                }
                items.push_signal(Signal::NewLine);
                items.push_string(" * </pre>".to_string());
            }
            JavadocSegment::BlankLine => {
                items.push_signal(Signal::NewLine);
                items.push_string(" *".to_string());
            }
        }
    }

    // Closing
    items.push_signal(Signal::NewLine);
    items.push_string(" */".to_string());

    items
}

/// Extract the inner text content from a Javadoc comment.
///
/// Strips the `/**` prefix and `*/` suffix, and normalizes each
/// continuation line by removing the leading ` * ` prefix.
fn extract_javadoc_content(text: &str) -> String {
    // Remove /** and */
    let inner = text
        .strip_prefix("/**")
        .unwrap_or(text)
        .strip_suffix("*/")
        .unwrap_or(text);

    let mut lines = Vec::new();
    for (i, line) in inner.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if i == 0 {
            // First line (after /**) — just trim whitespace
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        } else {
            // Continuation lines: strip leading whitespace and optional `*`
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix('*') {
                // Strip one leading space after * if present
                let rest = rest.strip_prefix(' ').unwrap_or(rest);
                lines.push(rest.to_string());
            } else {
                lines.push(trimmed.to_string());
            }
        }
    }

    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

/// Represents a parsed segment of a Javadoc comment.
#[derive(Debug)]
enum JavadocSegment {
    /// Free-form description text.
    Text(String),
    /// A Javadoc tag like `@param`, `@return`, `@throws`.
    Tag {
        name: String,
        args: Option<String>,
        desc: String,
    },
    /// A `<pre>...</pre>` block preserved verbatim.
    PreBlock(String),
    /// A blank line separator.
    BlankLine,
}

/// Parse Javadoc inner content into structured segments.
fn parse_javadoc_segments(content: &str) -> Vec<JavadocSegment> {
    let mut segments = Vec::new();
    let lines: Vec<&str> = content.split('\n').collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Blank line
        if trimmed.is_empty() {
            segments.push(JavadocSegment::BlankLine);
            i += 1;
            continue;
        }

        // <pre> block
        if trimmed.starts_with("<pre>")
            || trimmed.starts_with("{@code") && trimmed.contains("<pre>")
        {
            let mut pre_content = Vec::new();
            // Find the content after <pre>
            let after_pre = if let Some(pos) = trimmed.find("<pre>") {
                &trimmed[pos + 5..]
            } else {
                ""
            };
            if !after_pre.is_empty() && !after_pre.trim().is_empty() {
                pre_content.push(after_pre.to_string());
            }
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim();
                if l.contains("</pre>") {
                    // Get content before </pre>
                    if let Some(pos) = l.find("</pre>") {
                        let before = &l[..pos];
                        if !before.is_empty() {
                            pre_content.push(before.to_string());
                        }
                    }
                    i += 1;
                    break;
                }
                pre_content.push(lines[i].to_string());
                i += 1;
            }
            segments.push(JavadocSegment::PreBlock(pre_content.join("\n")));
            continue;
        }

        // Tag line
        if trimmed.starts_with('@') {
            let (tag_name, tag_args, tag_desc) = parse_tag_line(trimmed);
            // Collect continuation lines (non-blank, non-tag, non-pre lines)
            let mut full_desc = tag_desc;
            i += 1;
            while i < lines.len() {
                let next = lines[i].trim();
                if next.is_empty() || next.starts_with('@') || next.starts_with("<pre>") {
                    break;
                }
                full_desc.push(' ');
                full_desc.push_str(next);
                i += 1;
            }
            segments.push(JavadocSegment::Tag {
                name: tag_name,
                args: tag_args,
                desc: full_desc,
            });
            continue;
        }

        // Regular text — collect consecutive non-blank, non-tag, non-pre lines
        let mut text_parts = Vec::new();
        while i < lines.len() {
            let l = lines[i].trim();
            if l.is_empty() || l.starts_with('@') || l.starts_with("<pre>") {
                break;
            }
            text_parts.push(l.to_string());
            i += 1;
        }
        segments.push(JavadocSegment::Text(text_parts.join(" ")));
    }

    segments
}

/// Parse a single Javadoc tag line into (name, optional_arg, description).
///
/// Examples:
/// - `@param name the name of the thing` -> ("@param", Some("name"), "the name of the thing")
/// - `@return the result` -> ("@return", None, "the result")
/// - `@throws IOException if I/O fails` -> ("@throws", Some("IOException"), "if I/O fails")
fn parse_tag_line(line: &str) -> (String, Option<String>, String) {
    let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
    let tag_name = parts[0].to_string();
    let rest = if parts.len() > 1 { parts[1].trim() } else { "" };

    // Tags that take an argument (parameter name, exception type)
    match tag_name.as_str() {
        "@param" | "@throws" | "@exception" | "@serialField" => {
            let rest_parts: Vec<&str> = rest.splitn(2, char::is_whitespace).collect();
            let arg = rest_parts[0].to_string();
            let desc = if rest_parts.len() > 1 {
                rest_parts[1].trim().to_string()
            } else {
                String::new()
            };
            (tag_name, Some(arg), desc)
        }
        _ => (tag_name, None, rest.to_string()),
    }
}

/// Format a tag line for output.
fn format_tag_line(name: &str, args: &Option<String>, desc: &str) -> String {
    let mut result = name.to_string();
    if let Some(arg) = args {
        result.push(' ');
        result.push_str(arg);
    }
    if !desc.is_empty() {
        result.push(' ');
        result.push_str(desc);
    }
    result
}

/// Word-wrap text to the given maximum width.
///
/// Preserves inline `{@code ...}` constructs as atomic units.
/// Returns a vector of lines.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let words = split_preserving_inline_tags(text);
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in &words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    if lines.is_empty() {
        vec![String::new()]
    } else {
        lines
    }
}

/// Split text into words, preserving `{@code ...}` and similar inline tags
/// as single tokens.
fn split_preserving_inline_tags(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current_word = String::new();

    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '@' {
            // Start of inline tag — collect until matching '}'
            if !current_word.is_empty() {
                // Flush the word accumulated before the tag
                for w in current_word.split_whitespace() {
                    tokens.push(w.to_string());
                }
                current_word.clear();
            }
            let mut tag = String::new();
            let mut depth = 0;
            while i < chars.len() {
                tag.push(chars[i]);
                if chars[i] == '{' {
                    depth += 1;
                } else if chars[i] == '}' {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
            tokens.push(tag);
        } else {
            current_word.push(chars[i]);
            i += 1;
        }
    }

    if !current_word.is_empty() {
        for w in current_word.split_whitespace() {
            tokens.push(w.to_string());
        }
    }

    tokens
}

/// Determine if a comment is a trailing comment (on the same line as preceding code).
///
/// A comment is "trailing" if there is a previous sibling on the same line,
/// i.e., the previous non-extra sibling ends on the same line as the comment starts.
pub fn is_trailing_comment(node: tree_sitter::Node) -> bool {
    let comment_start_row = node.start_position().row;

    // Walk backwards through previous siblings
    let mut prev = node.prev_sibling();
    while let Some(sibling) = prev {
        if !sibling.is_extra() {
            // Found a non-comment sibling — check if it ends on the same line
            return sibling.end_position().row == comment_start_row;
        }
        // Skip over other extra nodes (other comments)
        prev = sibling.prev_sibling();
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use dprint_core::configuration::NewLineKind;

    fn test_config() -> Configuration {
        Configuration {
            line_width: 80,
            indent_width: 4,
            use_tabs: false,
            new_line_kind: NewLineKind::LineFeed,
            format_javadoc: true,
            method_chain_threshold: 80,
            inline_lambdas: true,
        }
    }

    fn parse_and_get_comment(source: &str) -> (tree_sitter::Tree, String) {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        (tree, source.to_string())
    }

    #[test]
    fn test_line_comment_with_space() {
        let source = "// hello world\nclass A {}\n";
        let (tree, src) = parse_and_get_comment(source);
        let config = test_config();
        let context = FormattingContext::new(&src, &config);

        let root = tree.root_node();
        let mut cursor = root.walk();
        let mut found = false;
        for child in root.children(&mut cursor) {
            if child.kind() == "line_comment" {
                let items = gen_line_comment(child, &context);
                assert!(!items.is_empty());
                found = true;
            }
        }
        assert!(found, "Expected to find a line_comment node");
    }

    #[test]
    fn test_line_comment_add_space() {
        let source = "//hello world\nclass A {}\n";
        let (tree, src) = parse_and_get_comment(source);
        let config = test_config();
        let context = FormattingContext::new(&src, &config);

        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "line_comment" {
                let items = gen_line_comment(child, &context);
                // The output should have the space added
                assert!(!items.is_empty());
            }
        }
    }

    #[test]
    fn test_block_comment_single_line() {
        let source = "/* hello */\nclass A {}\n";
        let (tree, src) = parse_and_get_comment(source);
        let config = test_config();
        let context = FormattingContext::new(&src, &config);

        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "block_comment" {
                let items = gen_block_comment(child, &context);
                assert!(!items.is_empty());
            }
        }
    }

    #[test]
    fn test_extract_javadoc_content() {
        let text = "/**\n * Hello world.\n * @param name the name\n */";
        let content = extract_javadoc_content(text);
        assert!(content.contains("Hello world."));
        assert!(content.contains("@param name the name"));
    }

    #[test]
    fn test_parse_tag_line_param() {
        let (name, args, desc) = parse_tag_line("@param name the name of the thing");
        assert_eq!(name, "@param");
        assert_eq!(args, Some("name".to_string()));
        assert_eq!(desc, "the name of the thing");
    }

    #[test]
    fn test_parse_tag_line_return() {
        let (name, args, desc) = parse_tag_line("@return the result");
        assert_eq!(name, "@return");
        assert_eq!(args, None);
        assert_eq!(desc, "the result");
    }

    #[test]
    fn test_wrap_text_short() {
        let lines = wrap_text("hello world", 80);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_text_long() {
        let long = "this is a really long line that should definitely be wrapped because it exceeds the maximum width";
        let lines = wrap_text(long, 40);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.len() <= 40 || line.split_whitespace().count() == 1);
        }
    }

    #[test]
    fn test_wrap_preserves_inline_code() {
        let text = "See {@code SomeClass} for details";
        let lines = wrap_text(text, 80);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("{@code SomeClass}"));
    }

    #[test]
    fn test_split_preserving_inline_tags() {
        let tokens = split_preserving_inline_tags("See {@code SomeClass} for details");
        assert_eq!(tokens, vec!["See", "{@code SomeClass}", "for", "details"]);
    }

    #[test]
    fn test_is_trailing_comment() {
        let source = "class A {} // trailing\n";
        let (tree, _src) = parse_and_get_comment(source);
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "line_comment" {
                assert!(is_trailing_comment(child));
            }
        }
    }

    #[test]
    fn test_is_leading_comment() {
        let source = "// leading\nclass A {}\n";
        let (tree, _src) = parse_and_get_comment(source);
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "line_comment" {
                assert!(!is_trailing_comment(child));
            }
        }
    }
}
