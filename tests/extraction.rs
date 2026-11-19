use i18n_mirror::{extract, render, Config};
use std::path::Path;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path:?}: {e}"))
}

/// Assert that the dummy line at the same 0-based index as the source line
/// containing `src_needle` contains `dummy_needle`. This is the core
/// line-number-preservation invariant: `#: file.tsx:LINE` references in the
/// final .po must point at the real call site.
fn assert_aligned(src: &str, dummy: &str, src_needle: &str, dummy_needle: &str) {
    let idx = src
        .lines()
        .position(|l| l.contains(src_needle))
        .unwrap_or_else(|| panic!("source needle {src_needle:?} not found"));
    let line = dummy.lines().nth(idx).unwrap_or("<end of file>");
    assert!(
        line.contains(dummy_needle),
        "dummy line {} ({idx}) is {line:?}, expected it to contain {dummy_needle:?}",
        idx + 1
    );
}

/// Parse the dummy output with the plain JavaScript grammar to prove it is
/// valid input for `xgettext --language=JavaScript`.
fn assert_valid_js(dummy: &str) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_javascript::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(dummy, None).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "dummy output is not valid JS:\n{dummy}\nerrors at: {}",
        format_errors(tree.root_node(), dummy)
    );
}

fn format_errors(node: tree_sitter::Node, src: &str) -> String {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.has_error() {
            if child.is_error() {
                let p = child.start_position();
                out.push(format!(
                    "line {} col {}: {:?}",
                    p.row + 1,
                    p.column + 1,
                    &src[child.byte_range()]
                ));
            } else {
                out.push(format_errors(child, src));
            }
        }
    }
    out.join("; ")
}

#[test]
fn tricky_tsx_extracts_all_shapes_with_aligned_lines() {
    let src = fixture("tricky.tsx");
    let ext = extract(&src, "tsx", &Config::default()).unwrap();

    assert!(!ext.had_parse_errors, "fixture must parse cleanly");
    assert_eq!(
        ext.calls.len(),
        13,
        "all call sites extracted: {:#?}",
        ext.calls
    );
    assert_eq!(
        ext.warnings.len(),
        2,
        "DYNAMIC and OPTS warn: {:#?}",
        ext.warnings
    );
    assert!(
        ext.warnings[0].contains("not a statically evaluable"),
        "{}",
        ext.warnings[0]
    );
    assert!(
        ext.warnings[1].contains("extra non-literal arguments"),
        "{}",
        ext.warnings[1]
    );

    let dummy = render(&ext);
    assert_valid_js(&dummy);

    // gettext / ngettext / pgettext / npgettext shapes, normalized.
    assert_aligned(
        &src,
        &dummy,
        r#"gettext("Checkout")"#,
        r#"gettext("Checkout");"#,
    );
    assert_aligned(
        &src,
        &dummy,
        r#"gettext("Fresh produce"#,
        r#"gettext("Fresh produce, fast delivery");"#,
    );
    assert_aligned(
        &src,
        &dummy,
        r#"ngettext("1 item""#,
        r#"ngettext("1 item", "%(count)s items", 1);"#,
    );
    assert_aligned(
        &src,
        &dummy,
        r#"npgettext("#,
        r#"npgettext("ice cream", "%(count)s scoop", "%(count)s scoops", 1);"#,
    );
    assert_aligned(
        &src,
        &dummy,
        r#"pgettext("cart""#,
        r#"pgettext("cart", "Shopping cart");"#,
    );

    // Template literal without substitutions, spanning source lines, still
    // lands on the line where the call starts.
    assert_aligned(
        &src,
        &dummy,
        "HELP_TEXT = t(",
        r#"gettext("Multi-line\ntemplate literal");"#,
    );

    // Concatenation is folded into one msgid.
    assert_aligned(
        &src,
        &dummy,
        "JOINED",
        r#"gettext("Chunks are joined together");"#,
    );

    // TS `as` cast is unwrapped.
    assert_aligned(
        &src,
        &dummy,
        r#"gettext("typed away""#,
        r#"gettext("typed away");"#,
    );

    // Untagged comment above SLOGAN is not attached anywhere.
    let slogan_line = src
        .lines()
        .position(|l| l.contains("Fresh produce"))
        .unwrap();
    let prev = dummy.lines().nth(slogan_line - 1).unwrap_or_default();
    assert!(
        !prev.contains("plain comment"),
        "untagged comment must not be copied"
    );

    // Tagged comments ARE copied, on their own original lines.
    assert_aligned(
        &src,
        &dummy,
        "// Translators: title",
        "// Translators: title of the checkout screen",
    );
    assert_aligned(
        &src,
        &dummy,
        "// Translators: items",
        "// Translators: items in the shopping cart",
    );
    assert_aligned(
        &src,
        &dummy,
        "/* Translators: block comment",
        "/* Translators: block comment",
    );
    assert_aligned(
        &src,
        &dummy,
        "spanning multiple lines",
        "spanning multiple lines */",
    );

    // Trailing same-line comment must not leak into the dummy.
    assert!(
        !dummy.contains("trailing comment"),
        "no trailing comment in dummy"
    );

    // A call directly after a trailing-comment line still extracts.
    assert_aligned(
        &src,
        &dummy,
        r#"gettext("Still works")"#,
        r#"gettext("Still works");"#,
    );

    // Idiomatic JSX brace-wrapped translator comments attach; tree-sitter's
    // comment node excludes the braces, so the dummy gets the pure comment.
    assert_aligned(
        &src,
        &dummy,
        r#"t("Brace wrapped comment")"#,
        r#"gettext("Brace wrapped comment");"#,
    );
    assert_aligned(
        &src,
        &dummy,
        "{/* Translators: idiomatic JSX",
        "/* Translators: idiomatic JSX translator comment */",
    );

    // The dummy is blank where nothing was extracted.
    let import_line = src.lines().position(|l| l.contains("import")).unwrap();
    assert_eq!(dummy.lines().nth(import_line), Some(""));
}

#[test]
fn modern_js_member_calls_and_quotes() {
    let src = fixture("modern.js");
    let ext = extract(&src, "js", &Config::default()).unwrap();

    assert!(!ext.had_parse_errors, "fixture must parse cleanly");
    assert_eq!(
        ext.calls.len(),
        6,
        "member calls via i18n.t / window.gettext / window.ngettext: {:#?}",
        ext.calls
    );

    let dummy = render(&ext);
    assert_valid_js(&dummy);

    assert_aligned(
        &src,
        &dummy,
        r#"i18n.t("Dashboard overview")"#,
        r#"gettext("Dashboard overview");"#,
    );
    assert_aligned(
        &src,
        &dummy,
        r#"window.ngettext("%(n)s product selected""#,
        r#"ngettext("%(n)s product selected", "%(n)s products selected", 1);"#,
    );
    // `?.` chains still count as member calls on the property name.
    assert_aligned(
        &src,
        &dummy,
        r#"t?.("Deeply optional")"#,
        r#"gettext("Deeply optional");"#,
    );
    // Quote styles normalize to double quotes, escapes re-encoded.
    assert_aligned(
        &src,
        &dummy,
        r#"_("He said \"hello\"")"#,
        r#"gettext("He said \"hello\"");"#,
    );
    assert_aligned(
        &src,
        &dummy,
        r"'it\'s single-quoted'",
        r#"gettext("it's single-quoted");"#,
    );
    assert_aligned(
        &src,
        &dummy,
        r#"gettext("Цената е"#,
        r#"gettext("Цената е %(price)s лв.");"#,
    );

    // Tagged indented comment is attached.
    assert_aligned(
        &src,
        &dummy,
        "// Translators: heading",
        "// Translators: heading on the dashboard card",
    );
}

#[test]
fn ignores_unconfigured_functions_and_wrong_arity() {
    let src = r#"
        translate("not configured");
        rot("not configured");
        gettext("ok");
        ngettext("only one arg");
        pgettext("missing msgid");
    "#;
    let ext = extract(src, "js", &Config::default()).unwrap();
    assert_eq!(ext.calls.len(), 1);
    assert_eq!(ext.warnings.len(), 2, "{:#?}", ext.warnings);
}

#[test]
fn two_calls_on_one_line_share_it() {
    let src = "const a = _('first'); const b = _('second');\n";
    let ext = extract(src, "js", &Config::default()).unwrap();
    assert_eq!(ext.calls.len(), 2);
    let dummy = render(&ext);
    assert_eq!(
        dummy.lines().next(),
        Some(r#"gettext("first"); gettext("second");"#)
    );
    assert_valid_js(&dummy);
}

#[test]
fn template_with_substitution_is_rejected() {
    // Regression: tree-sitter names the interpolation node
    // `template_substitution` — templates containing one must warn and skip,
    // never leak a literal "${…}" into a msgid.
    let src =
        "const a = t(`Hi ${name}`);\nconst b = t(`Hi ${'you'}`);\nconst c = t(`Plain template`);\n";
    let ext = extract(src, "js", &Config::default()).unwrap();
    assert_eq!(ext.calls.len(), 1);
    assert_eq!(ext.calls[0].code, r#"gettext("Plain template");"#);
    assert_eq!(ext.warnings.len(), 2, "{:#?}", ext.warnings);
    assert!(
        !render(&ext).contains("${"),
        "no interpolation may leak into dummy output"
    );

    // Also inside concatenation.
    let src2 = "const d = t(\"Hi \" + `${name}` + \"!\");\n";
    let ext2 = extract(src2, "js", &Config::default()).unwrap();
    assert!(ext2.calls.is_empty());
    assert_eq!(ext2.warnings.len(), 1);
}

#[test]
fn custom_function_table() {
    let mut config = Config::default();
    config.functions.clear();
    config
        .functions
        .insert("tr".to_string(), i18n_mirror::Kind::Gettext);
    let src = r#"tr("custom fn"); gettext("default fn no longer configured");"#;
    let ext = extract(src, "js", &config).unwrap();
    assert_eq!(ext.calls.len(), 1);
    assert_eq!(ext.calls[0].code, r#"gettext("custom fn");"#);
}
