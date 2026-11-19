//! Extraction over unmodified files from real open-source React projects
//! (see tests/realworld/SOURCES.md for provenance and licenses).
//!
//! These cannot change under us (they are vendored), so the expected numbers
//! below are stable facts about the fixture files.

use i18n_mirror::{extract, render, Config};
use std::path::{Path, PathBuf};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/realworld")
}

fn run(name: &str, ext: &str) -> i18n_mirror::Extraction {
    let path = dir().join(name);
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let extraction = extract(&src, ext, &Config::default())
        .unwrap_or_else(|e| panic!("{name}: extract failed: {e}"));
    assert!(
        !extraction.had_parse_errors,
        "{name}: real-world file must parse without errors"
    );
    extraction
}

#[test]
fn outline_settings_tsx() {
    let ext = run("outline-settings.tsx", "tsx");
    assert_eq!(ext.calls.len(), 7, "{:#?}", ext.calls);
    assert!(ext.warnings.is_empty(), "{:#?}", ext.warnings);

    let dummy = render(&ext);
    assert_valid_js(&dummy);

    let ids: Vec<&str> = ext.calls.iter().map(|c| c.code.as_str()).collect();
    assert!(ids.contains(&r#"gettext("Toggle theme");"#), "{ids:#?}");
    // Two calls on a single source line end up on a single dummy line.
    let src = std::fs::read_to_string(dir().join("outline-settings.tsx")).unwrap();
    let row = src
        .lines()
        .position(|l| l.contains("Appearance") && l.contains("Change theme"))
        .unwrap();
    let line = dummy.lines().nth(row).unwrap();
    assert!(line.contains(r#"gettext("Appearance");"#), "{line:?}");
    assert!(line.contains(r#"gettext("Change theme");"#), "{line:?}");
}

#[test]
fn outline_api_keys_tsx() {
    let ext = run("outline-api-keys.tsx", "tsx");
    assert_eq!(ext.calls.len(), 5, "{:#?}", ext.calls);
    assert!(ext.warnings.is_empty(), "{:#?}", ext.warnings);
    assert_valid_js(&render(&ext));
}

#[test]
fn excalidraw_menu_tsx() {
    let ext = run("excalidraw-app-main-menu.tsx", "tsx");
    assert_eq!(ext.calls.len(), 2, "{:#?}", ext.calls);
    assert!(ext.warnings.is_empty(), "{:#?}", ext.warnings);
    let dummy = render(&ext);
    assert_valid_js(&dummy);
    assert!(dummy.contains(r#"gettext("labels.signIn");"#));
}

#[test]
fn react_admin_page_tsx() {
    // Modern syntax (import.meta.env etc.) parses; the file has no literal
    // translation calls, so nothing is extracted and nothing is warned about.
    let ext = run("react-admin-settings-page.tsx", "tsx");
    assert!(ext.calls.is_empty(), "{:#?}", ext.calls);
    assert!(ext.warnings.is_empty(), "{:#?}", ext.warnings);
}

#[test]
fn zulip_compose_closed_ui_ts() {
    // 13 KB of TS using $t({defaultMessage: …}) object-literal i18n: parses
    // cleanly, extracts nothing, warns nothing.
    let ext = run("zulip-compose-closed-ui.ts", "ts");
    assert!(ext.calls.is_empty(), "{:#?}", ext.calls);
    assert!(ext.warnings.is_empty(), "{:#?}", ext.warnings);
}

fn assert_valid_js(dummy: &str) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(dummy, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "dummy output is not valid JS:\n{dummy}"
    );
}
