//! Stability test against the 10 Jahia files from issue #1.
//! These are the files that triggered "Formatting not stable. Bailed after 5 tries."
//! Ignored by default (requires cloning Jahia repo to /tmp/jahia).

use std::path::Path;

use dprint_core::configuration::NewLineKind;
use dprint_plugin_java::configuration::Configuration;
use dprint_plugin_java::format_text::format_text;

fn config() -> Configuration {
    Configuration {
        line_width: 120,
        indent_width: 4,
        use_tabs: false,
        new_line_kind: NewLineKind::LineFeed,
        format_javadoc: true,
        method_chain_threshold: 80,
        inline_lambdas: true,
    }
}

fn assert_stable(path: &str) {
    let source = std::fs::read_to_string(path).unwrap();
    let config = config();
    let p = Path::new(path);
    let pass1 = format_text(p, &source, &config)
        .unwrap()
        .unwrap_or_else(|| source.clone());
    let pass2 = format_text(p, &pass1, &config)
        .unwrap()
        .unwrap_or_else(|| pass1.clone());
    assert!(pass1 == pass2, "Formatting not stable for {path}");
}

#[test]
#[ignore]
fn jahia_pom_utils() {
    assert_stable("/tmp/jahia/core/src/main/java/org/jahia/utils/PomUtils.java");
}

#[test]
#[ignore]
fn jahia_jcr_store_service() {
    assert_stable("/tmp/jahia/core/src/main/java/org/jahia/services/content/JCRStoreService.java");
}

#[test]
#[ignore]
fn jahia_template_package_deployer() {
    assert_stable(
        "/tmp/jahia/core/src/main/java/org/jahia/services/templates/TemplatePackageDeployer.java",
    );
}

#[test]
#[ignore]
fn jahia_sites_service() {
    assert_stable("/tmp/jahia/core/src/main/java/org/jahia/services/sites/JahiaSitesService.java");
}

#[test]
#[ignore]
fn jahia_login_module() {
    assert_stable(
        "/tmp/jahia/core/src/main/java/org/apache/jackrabbit/core/security/JahiaLoginModule.java",
    );
}

#[test]
#[ignore]
fn jahia_node_helper() {
    assert_stable("/tmp/jahia/gwt/src/main/java/org/jahia/ajax/gwt/helper/NodeHelper.java");
}

#[test]
#[ignore]
fn jahia_info_tab_item() {
    assert_stable(
        "/tmp/jahia/gwt/src/main/java/org/jahia/ajax/gwt/client/widget/content/InfoTabItem.java",
    );
}

#[test]
#[ignore]
fn jahia_content_type_window() {
    assert_stable(
        "/tmp/jahia/gwt/src/main/java/org/jahia/ajax/gwt/client/widget/edit/ContentTypeWindow.java",
    );
}

#[test]
#[ignore]
fn jahia_content_tab_item() {
    assert_stable(
        "/tmp/jahia/gwt/src/main/java/org/jahia/ajax/gwt/client/widget/contentengine/ContentTabItem.java",
    );
}

#[test]
#[ignore]
fn jahia_pages_tab_item() {
    assert_stable(
        "/tmp/jahia/gwt/src/main/java/org/jahia/ajax/gwt/client/widget/edit/sidepanel/PagesTabItem.java",
    );
}
