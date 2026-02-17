use crate::configuration::Configuration;

/// Formatting context that tracks state during CST traversal.
///
/// This holds the configuration, source text reference, and mutable
/// state like the current indentation level and parent node stack
/// for context-aware formatting decisions.
#[allow(clippy::struct_excessive_bools)]
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
    /// that don't affect the base `indent_level` but need to be accounted for
    /// in width calculations for nested argument lists.
    continuation_indent_levels: usize,

    /// Set when a `variable_declarator` or `assignment_expression` has wrapped at '='.
    /// When true, the chain wrapper knows the LHS prefix is on the previous line
    /// and should use `prefix_width=0` with continuation indent.
    assignment_wrapped: bool,

    /// Override prefix width for the next `formal_parameters` call.
    /// Used when method name wraps to continuation line, making the effective
    /// prefix shorter than what `estimate_prefix_width` computes from source.
    override_prefix_width: Option<usize>,

    /// Track whether a type argument list wrapped while emitting a declaration type.
    /// This is used to align the subsequent variable declarator on a continuation line.
    track_type_args_wrapping: bool,
    type_args_wrapped: bool,

    /// Indicates the current variable declarator starts on a continuation line
    /// (for example, after a wrapped generic type).
    declarator_on_new_line: bool,
}

impl<'a> FormattingContext<'a> {
    /// Create a new formatting context.
    #[must_use]
    pub fn new(source: &'a str, config: &'a Configuration) -> Self {
        Self {
            source,
            config,
            indent_level: 0,
            parent_stack: Vec::new(),
            continuation_indent_levels: 0,
            assignment_wrapped: false,
            override_prefix_width: None,
            track_type_args_wrapping: false,
            type_args_wrapped: false,
            declarator_on_new_line: false,
        }
    }

    /// Get the current indentation level.
    #[must_use]
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
    #[must_use]
    pub fn parent(&self) -> Option<&'static str> {
        self.parent_stack.last().copied()
    }

    /// Check if the given node kind is in the parent stack.
    #[must_use]
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
    #[must_use]
    pub fn effective_indent_level(&self) -> usize {
        self.indent_level + self.continuation_indent_levels
    }

    /// Set the `assignment_wrapped` flag.
    pub fn set_assignment_wrapped(&mut self, wrapped: bool) {
        self.assignment_wrapped = wrapped;
    }

    /// Check if the current chain is inside an assignment that already wrapped at '='.
    #[must_use]
    pub fn is_assignment_wrapped(&self) -> bool {
        self.assignment_wrapped
    }

    /// Set override prefix width for the next `formal_parameters` call.
    pub fn set_override_prefix_width(&mut self, width: Option<usize>) {
        self.override_prefix_width = width;
    }

    /// Take (consume) the override prefix width, if any.
    pub fn take_override_prefix_width(&mut self) -> Option<usize> {
        self.override_prefix_width.take()
    }

    /// Begin tracking whether a type argument list wraps while emitting a declaration type.
    pub fn start_type_args_wrap_tracking(&mut self) {
        self.track_type_args_wrapping = true;
        self.type_args_wrapped = false;
    }

    /// Mark that a type argument list wrapped.
    pub fn mark_type_args_wrapped(&mut self) {
        if self.track_type_args_wrapping {
            self.type_args_wrapped = true;
        }
    }

    /// Stop tracking and return whether any type arguments wrapped.
    pub fn finish_type_args_wrap_tracking(&mut self) -> bool {
        self.track_type_args_wrapping = false;
        std::mem::take(&mut self.type_args_wrapped)
    }

    /// Set whether the current variable declarator starts on a continuation line.
    pub fn set_declarator_on_new_line(&mut self, value: bool) {
        self.declarator_on_new_line = value;
    }

    /// Check whether the current variable declarator starts on a continuation line.
    #[must_use]
    pub fn is_declarator_on_new_line(&self) -> bool {
        self.declarator_on_new_line
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
