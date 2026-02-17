use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::ParseConfigurationError;
use serde::Deserialize;
use serde::Serialize;

/// Formatting style presets inspired by palantir-java-format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JavaStyle {
    /// 120-char line width, 4-space indent (palantir-java-format default).
    Palantir,
    /// 100-char line width, 2-space indent (google-java-format default).
    Google,
    /// 100-char line width, 4-space indent (Android Open Source Project).
    Aosp,
}

dprint_core::generate_str_to_from![
    JavaStyle,
    [Palantir, "palantir"],
    [Google, "google"],
    [Aosp, "aosp"]
];

impl JavaStyle {
    #[must_use]
    pub fn line_width(self) -> u32 {
        match self {
            JavaStyle::Palantir => 120,
            JavaStyle::Google | JavaStyle::Aosp => 100,
        }
    }

    #[must_use]
    pub fn indent_width(self) -> u8 {
        match self {
            JavaStyle::Palantir | JavaStyle::Aosp => 4,
            JavaStyle::Google => 2,
        }
    }
}

/// Resolved configuration for the Java formatter plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    /// Maximum line width before wrapping.
    pub line_width: u32,
    /// Number of spaces per indentation level.
    pub indent_width: u8,
    /// Whether to use tabs instead of spaces.
    pub use_tabs: bool,
    /// Newline character to use.
    pub new_line_kind: NewLineKind,
    /// Whether to format Javadoc comments.
    pub format_javadoc: bool,
    /// Character threshold at which method chains get broken across lines.
    /// Lines with chained method calls exceeding this width will be wrapped.
    pub method_chain_threshold: u32,
    /// Whether to prefer inlining lambdas on a single line when they fit.
    pub inline_lambdas: bool,
}
