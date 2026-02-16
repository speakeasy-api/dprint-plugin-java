use crate::configuration::Configuration;

/// Formatting context that tracks state during CST traversal.
///
/// This holds the configuration, source text reference, and mutable
/// state like the current indentation level and parent node stack
/// for context-aware formatting decisions.
pub struct FormattingContext<'a> {
    /// Reference to the source text being formatted.
    pub source: &'a str,

    /// Formatter configuration settings.
    pub config: &'a Configuration,

    /// Current indentation level (0-based).
    indent_level: usize,

    /// Stack of parent node kinds for context-aware formatting.
    /// The top of the stack is the immediate parent.
    parent_stack: Vec<&'static str>,

    /// Additional continuation indent levels (for chain wrapping).
    /// When a method chain wraps, we add +2 levels of continuation indent
    /// that don't affect the base indent_level but need to be accounted for
    /// in width calculations for nested argument lists.
    continuation_indent_levels: usize,
}

impl<'a> FormattingContext<'a> {
    /// Create a new formatting context.
    pub fn new(source: &'a str, config: &'a Configuration) -> Self {
        Self {
            source,
            config,
            indent_level: 0,
            parent_stack: Vec::new(),
            continuation_indent_levels: 0,
        }
    }

    /// Get the current indentation level.
    pub fn indent_level(&self) -> usize {
        self.indent_level
    }

    /// Increase the indentation level by one.
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease the indentation level by one.
    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    /// Push a parent node kind onto the stack.
    pub fn push_parent(&mut self, kind: &'static str) {
        self.parent_stack.push(kind);
    }

    /// Pop a parent node kind from the stack.
    pub fn pop_parent(&mut self) {
        self.parent_stack.pop();
    }

    /// Get the immediate parent node kind, if any.
    pub fn parent(&self) -> Option<&'static str> {
        self.parent_stack.last().copied()
    }

    /// Check if the given node kind is in the parent stack.
    pub fn has_ancestor(&self, kind: &'static str) -> bool {
        self.parent_stack.contains(&kind)
    }

    /// Add continuation indent levels (for wrapped chains).
    pub fn add_continuation_indent(&mut self, levels: usize) {
        self.continuation_indent_levels += levels;
    }

    /// Remove continuation indent levels (for wrapped chains).
    pub fn remove_continuation_indent(&mut self, levels: usize) {
        if self.continuation_indent_levels >= levels {
            self.continuation_indent_levels -= levels;
        }
    }

    /// Get the effective indent level including continuation indent.
    pub fn effective_indent_level(&self) -> usize {
        self.indent_level + self.continuation_indent_levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::Configuration;
    use dprint_core::configuration::NewLineKind;

    fn test_config() -> Configuration {
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
    fn test_indent_dedent() {
        let config = test_config();
        let mut ctx = FormattingContext::new("", &config);

        assert_eq!(ctx.indent_level(), 0);

        ctx.indent();
        assert_eq!(ctx.indent_level(), 1);

        ctx.indent();
        assert_eq!(ctx.indent_level(), 2);

        ctx.dedent();
        assert_eq!(ctx.indent_level(), 1);

        ctx.dedent();
        assert_eq!(ctx.indent_level(), 0);

        // Dedent at 0 should stay at 0
        ctx.dedent();
        assert_eq!(ctx.indent_level(), 0);
    }

    #[test]
    fn test_parent_stack() {
        let config = test_config();
        let mut ctx = FormattingContext::new("", &config);

        assert_eq!(ctx.parent(), None);
        assert!(!ctx.has_ancestor("class_declaration"));

        ctx.push_parent("class_declaration");
        assert_eq!(ctx.parent(), Some("class_declaration"));
        assert!(ctx.has_ancestor("class_declaration"));

        ctx.push_parent("method_declaration");
        assert_eq!(ctx.parent(), Some("method_declaration"));
        assert!(ctx.has_ancestor("class_declaration"));
        assert!(ctx.has_ancestor("method_declaration"));

        ctx.pop_parent();
        assert_eq!(ctx.parent(), Some("class_declaration"));
        assert!(ctx.has_ancestor("class_declaration"));
        assert!(!ctx.has_ancestor("method_declaration"));

        ctx.pop_parent();
        assert_eq!(ctx.parent(), None);
    }
}
