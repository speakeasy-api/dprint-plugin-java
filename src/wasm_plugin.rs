use anyhow::Result;
use dprint_core::configuration::ConfigKeyMap;
use dprint_core::configuration::GlobalConfiguration;
use dprint_core::generate_plugin_code;
use dprint_core::plugins::CheckConfigUpdatesMessage;
use dprint_core::plugins::ConfigChange;
use dprint_core::plugins::FileMatchingInfo;
use dprint_core::plugins::FormatResult;
use dprint_core::plugins::PluginInfo;
use dprint_core::plugins::PluginResolveConfigurationResult;
use dprint_core::plugins::SyncFormatRequest;
use dprint_core::plugins::SyncHostFormatRequest;
use dprint_core::plugins::SyncPluginHandler;

use crate::configuration::resolve_config;
use crate::configuration::Configuration;

struct JavaPluginHandler;

impl SyncPluginHandler<Configuration> for JavaPluginHandler {
    fn plugin_info(&mut self) -> PluginInfo {
        let version = env!("CARGO_PKG_VERSION").to_string();
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_string(),
            version,
            config_key: "java".to_string(),
            help_url: "https://github.com/speakeasy-api/dprint-plugin-java".to_string(),
            config_schema_url: String::new(),
            update_url: None,
        }
    }

    fn license_text(&mut self) -> String {
        include_str!("../LICENSE").to_string()
    }

    fn resolve_config(
        &mut self,
        config: ConfigKeyMap,
        global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<Configuration> {
        let result = resolve_config(config, global_config);
        PluginResolveConfigurationResult {
            config: result.config,
            diagnostics: result.diagnostics,
            file_matching: FileMatchingInfo {
                file_extensions: vec!["java".to_string()],
                file_names: vec![],
            },
        }
    }

    fn check_config_updates(
        &self,
        _message: CheckConfigUpdatesMessage,
    ) -> Result<Vec<ConfigChange>> {
        Ok(Vec::new())
    }

    fn format(
        &mut self,
        request: SyncFormatRequest<Configuration>,
        _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        let file_text = String::from_utf8(request.file_bytes)?;
        crate::format_text(request.file_path, &file_text, request.config)
            .map(|maybe| maybe.map(|s| s.into_bytes()))
    }
}

generate_plugin_code!(JavaPluginHandler, JavaPluginHandler);
