use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::configuration::NewLineKind;
use dprint_core::configuration::ResolveConfigurationResult;
use dprint_core::configuration::get_unknown_property_diagnostics;
use dprint_core::configuration::get_value;

use super::Configuration;
use super::JavaStyle;

/// Resolve raw configuration key-value pairs into a typed `Configuration`.
pub fn resolve_config(
    config: ConfigKeyMap,
    global_config: &GlobalConfiguration,
) -> ResolveConfigurationResult<Configuration> {
    let mut config = config;
    let mut diagnostics = Vec::new();

    let style: JavaStyle = get_value(&mut config, "style", JavaStyle::Palantir, &mut diagnostics);

    let line_width = get_value(
        &mut config,
        "lineWidth",
        global_config.line_width.unwrap_or(style.line_width()),
        &mut diagnostics,
    );
    let indent_width = get_value(
        &mut config,
        "indentWidth",
        global_config.indent_width.unwrap_or(style.indent_width()),
        &mut diagnostics,
    );
    let use_tabs = get_value(
        &mut config,
        "useTabs",
        global_config.use_tabs.unwrap_or(false),
        &mut diagnostics,
    );
    let new_line_kind = get_value(
        &mut config,
        "newLineKind",
        global_config.new_line_kind.unwrap_or(NewLineKind::LineFeed),
        &mut diagnostics,
    );
    let format_javadoc = get_value(&mut config, "formatJavadoc", false, &mut diagnostics);
    let method_chain_threshold =
        get_value(&mut config, "methodChainThreshold", 80u32, &mut diagnostics);
    let inline_lambdas = get_value(&mut config, "inlineLambdas", true, &mut diagnostics);

    diagnostics.extend(get_unknown_property_diagnostics(config));

    ResolveConfigurationResult {
        config: Configuration {
            line_width,
            indent_width,
            use_tabs,
            new_line_kind,
            format_javadoc,
            method_chain_threshold,
            inline_lambdas,
        },
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dprint_core::configuration::ConfigKeyValue;

    #[test]
    fn default_palantir_style() {
        let config = ConfigKeyMap::new();
        let global = GlobalConfiguration::default();
        let result = resolve_config(config, &global);
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.config.line_width, 120);
        assert_eq!(result.config.indent_width, 4);
        assert!(!result.config.use_tabs);
        assert!(result.config.inline_lambdas);
        assert_eq!(result.config.method_chain_threshold, 80);
    }

    #[test]
    fn google_style_overrides() {
        let config =
            ConfigKeyMap::from([("style".to_string(), ConfigKeyValue::from_str("google"))]);
        let global = GlobalConfiguration::default();
        let result = resolve_config(config, &global);
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.config.line_width, 100);
        assert_eq!(result.config.indent_width, 2);
    }

    #[test]
    fn explicit_values_override_style() {
        let config = ConfigKeyMap::from([
            ("style".to_string(), ConfigKeyValue::from_str("google")),
            ("lineWidth".to_string(), ConfigKeyValue::from_i32(80)),
        ]);
        let global = GlobalConfiguration::default();
        let result = resolve_config(config, &global);
        assert!(result.diagnostics.is_empty());
        assert_eq!(result.config.line_width, 80);
        assert_eq!(result.config.indent_width, 2);
    }

    #[test]
    fn unknown_property_diagnostic() {
        let config =
            ConfigKeyMap::from([("unknownProp".to_string(), ConfigKeyValue::from_str("value"))]);
        let global = GlobalConfiguration::default();
        let result = resolve_config(config, &global);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].property_name, "unknownProp");
    }
}
