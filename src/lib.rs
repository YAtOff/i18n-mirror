//! i18n-mirror: extract gettext-style translation calls from modern JS/TS/JSX
//! sources and mirror them into "dummy" plain-JS files that GNU xgettext can
//! parse, preserving original file names and line numbers.
//!
//! Django's `makemessages -d djangojs` shells out to `xgettext --language=JavaScript`,
//! which cannot parse JSX, TypeScript, or much of modern ES syntax. Instead of
//! fighting xgettext, we generate a parallel tree of trivially-parseable files
//! (`gettext("…");` lines placed on their original source lines) so the
//! resulting `.po` references (`#: frontend/src/App.tsx:42`) point at the real
//! source with exact line numbers.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tree_sitter::{Language, Node, Parser, Query, QueryCursor, StreamingIterator};

pub mod js_string;

/// The four gettext call shapes xgettext understands (and Django's `djangojs`
/// domain configures keywords for).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// `gettext(msgid)`
    Gettext,
    /// `ngettext(singular, plural, n)`
    Ngettext,
    /// `pgettext(msgctxt, msgid)`
    Pgettext,
    /// `npgettext(msgctxt, singular, plural, n)`
    Npgettext,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        match s.trim().to_ascii_lowercase().as_str() {
            "gettext" => Some(Kind::Gettext),
            "ngettext" => Some(Kind::Ngettext),
            "pgettext" => Some(Kind::Pgettext),
            "npgettext" => Some(Kind::Npgettext),
            _ => None,
        }
    }

    /// Number of leading literal string arguments this shape requires
    /// (the trailing count argument of ngettext-style calls is ignored).
    fn literal_arity(self) -> usize {
        match self {
            Kind::Gettext => 1,
            Kind::Ngettext => 2,
            Kind::Pgettext => 2,
            Kind::Npgettext => 3,
        }
    }
}

/// Extraction configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Map of function name (or member-call property name, e.g. `t` in
    /// `i18n.t(…)`) to the gettext shape it should be treated as.
    pub functions: HashMap<String, Kind>,
    /// File extensions to scan (lowercase, without the dot).
    pub extensions: HashSet<String>,
}

pub const DEFAULT_FUNCTIONS: &[(&str, Kind)] = &[
    ("gettext", Kind::Gettext),
    ("gettext_noop", Kind::Gettext),
    ("gettext_lazy", Kind::Gettext),
    ("_", Kind::Gettext),
    ("t", Kind::Gettext),
    ("ngettext", Kind::Ngettext),
    ("ngettext_lazy", Kind::Ngettext),
    ("pgettext", Kind::Pgettext),
    ("pgettext_lazy", Kind::Pgettext),
    ("npgettext", Kind::Npgettext),
    ("npgettext_lazy", Kind::Npgettext),
];

impl Default for Config {
    fn default() -> Self {
        Config {
            functions: DEFAULT_FUNCTIONS
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
            extensions: ["js", "mjs", "cjs", "jsx", "ts", "mts", "cts", "tsx"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

/// A single extracted call site ready for dummy-file emission.
#[derive(Debug, Clone)]
pub struct DummyCall {
    /// Zero-based row of the *call's first line* in the original source.
    pub row: usize,
    /// Lines of the attached `Translators:` comment: (zero-based row, verbatim text).
    pub comment: Vec<(usize, String)>,
    /// The canonical dummy call, e.g. `ngettext("item", "items", 1);`
    pub code: String,
}

/// Result of extracting one file.
#[derive(Debug, Default, Clone)]
pub struct Extraction {
    pub calls: Vec<DummyCall>,
    pub warnings: Vec<String>,
    /// The source had tree-sitter ERROR nodes; results may be incomplete.
    pub had_parse_errors: bool,
}

impl Extraction {
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }
}

/// Grammar to use for a file extension.
fn grammar_for(ext: &str) -> Option<Language> {
    match ext {
        "js" | "mjs" | "cjs" | "jsx" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" | "mts" | "cts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        _ => None,
    }
}

fn build_query(lang: Language) -> Result<Query, String> {
    // Two patterns: translation-shaped calls (callee + arguments), and all
    // comments (used for Translators: attachment). No query predicates are
    // used on purpose — the Rust bindings do not reliably evaluate #match?,
    // so callee filtering happens in Rust against the configured name table.
    let src = r#"
        (call_expression
          function: [
            (identifier) @fn
            (member_expression property: (property_identifier) @fn)
          ]
          arguments: (arguments) @args) @call

        (comment) @comment
    "#;
    Query::new(&lang, src).map_err(|e| e.to_string())
}

/// Extract translation calls from one source file.
pub fn extract(source: &str, ext: &str, config: &Config) -> Result<Extraction, String> {
    let lang = grammar_for(ext).ok_or_else(|| format!("unsupported extension .{ext}"))?;
    let mut parser = Parser::new();
    parser.set_language(&lang).map_err(|e| e.to_string())?;
    let query = build_query(lang)?;

    let tree = parser
        .parse(source, None)
        .ok_or("parser returned no tree")?;
    let mut cursor = QueryCursor::new();

    let fn_idx = query.capture_index_for_name("fn").unwrap();
    let args_idx = query.capture_index_for_name("args").unwrap();
    let call_idx = query.capture_index_for_name("call").unwrap();
    let comment_idx = query.capture_index_for_name("comment").unwrap();

    let bytes = source.as_bytes();

    // Collect all comments up front: (start_row, start_col, end_row, text).
    let mut comments: Vec<(usize, usize, usize, String)> = Vec::new();
    // Raw call matches: (start_row of call, fn text, args node).
    let mut call_matches: Vec<(usize, String, Node)> = Vec::new();

    let mut matches = cursor.matches(&query, tree.root_node(), bytes);
    while let Some(m) = matches.next() {
        let mut fn_node = None;
        let mut args_node = None;
        let mut call_node = None;
        let mut comment_node = None;
        for cap in m.captures.iter() {
            if cap.index == fn_idx {
                fn_node = Some(cap.node);
            } else if cap.index == args_idx {
                args_node = Some(cap.node);
            } else if cap.index == call_idx {
                call_node = Some(cap.node);
            } else if cap.index == comment_idx {
                comment_node = Some(cap.node);
            }
        }
        if let Some(node) = comment_node {
            let text = node
                .utf8_text(bytes)
                .map_err(|_| "file is not valid UTF-8".to_string())?
                .to_string();
            comments.push((
                node.start_position().row,
                node.start_position().column,
                node.end_position().row,
                text,
            ));
            continue;
        }
        let (Some(f), Some(a), Some(c)) = (fn_node, args_node, call_node) else {
            continue;
        };
        let name = f
            .utf8_text(bytes)
            .map_err(|_| "not UTF-8".to_string())?
            .to_string();
        call_matches.push((c.start_position().row, name, a));
    }

    // Deterministic order: top to bottom, left to right.
    call_matches.sort_by_key(|(row, _, args)| (*row, args.start_position().column));

    let mut out = Extraction {
        had_parse_errors: tree.root_node().has_error(),
        ..Extraction::default()
    };

    for (row, name, args_node) in call_matches {
        let Some(kind) = config.functions.get(&name) else {
            continue;
        };
        let arg_nodes = positional_args(args_node);
        let arity = kind.literal_arity();

        if arg_nodes.len() < arity {
            out.warnings.push(format!(
                "line {}: {}() expects {} literal argument(s), found {} — skipped",
                row + 1,
                name,
                arity,
                arg_nodes.len()
            ));
            continue;
        }
        if arg_nodes.len() > arity && matches!(kind, Kind::Gettext) {
            // gettext with extra args (e.g. i18next `t("key", { ns })`) cannot
            // be represented in gettext without losing information.
            out.warnings.push(format!(
                "line {}: {}(\"…\", …) has extra non-literal arguments (interpolation options?) — skipped",
                row + 1,
                name
            ));
            continue;
        }

        let mut literals = Vec::with_capacity(arity);
        let mut ok = true;
        for arg in &arg_nodes[..arity] {
            match literal_string(*arg, bytes) {
                Some(s) => literals.push(s),
                None => {
                    out.warnings.push(format!(
                        "line {}: {}() argument is not a statically evaluable string literal — skipped",
                        row + 1,
                        name
                    ));
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }

        let code = match kind {
            Kind::Gettext => format!("gettext({});", js_string::encode(&literals[0])),
            Kind::Ngettext => format!(
                "ngettext({}, {}, 1);",
                js_string::encode(&literals[0]),
                js_string::encode(&literals[1])
            ),
            Kind::Pgettext => format!(
                "pgettext({}, {});",
                js_string::encode(&literals[0]),
                js_string::encode(&literals[1])
            ),
            Kind::Npgettext => format!(
                "npgettext({}, {}, {}, 1);",
                js_string::encode(&literals[0]),
                js_string::encode(&literals[1]),
                js_string::encode(&literals[2])
            ),
        };

        let comment = translator_comment_for(row, &comments, source).unwrap_or_default();

        out.calls.push(DummyCall { row, comment, code });
    }

    Ok(out)
}

/// Positional (non-comment) named children of an `arguments` node.
fn positional_args(args: Node) -> Vec<Node> {
    let mut v = Vec::new();
    for i in 0..args.child_count() {
        if let Some(c) = args.child(i) {
            if c.is_named() && c.kind() != "comment" && c.kind() != "optional_comma" {
                v.push(c);
            }
        }
    }
    v
}

/// Statically evaluate an expression to a string literal, or None.
fn literal_string(node: Node, bytes: &[u8]) -> Option<String> {
    match node.kind() {
        "string" => {
            let t = node.utf8_text(bytes).ok()?;
            let inner = &t[1..t.len() - 1];
            js_string::decode(inner, false)
        }
        "template_string" => {
            // Reject templates containing substitutions: `` t(`Hi ${name}`) ``
            for i in 0..node.child_count() {
                if let Some(c) = node.child(i) {
                    if c.kind() == "template_substitution" {
                        return None;
                    }
                }
            }
            let t = node.utf8_text(bytes).ok()?;
            let inner = &t[1..t.len() - 1];
            js_string::decode(inner, true)
        }
        "binary_expression" => {
            let op = node.child_by_field_name("operator")?;
            if op.utf8_text(bytes).ok()? != "+" {
                return None;
            }
            let lhs = literal_string(node.child_by_field_name("left")?, bytes)?;
            let rhs = literal_string(node.child_by_field_name("right")?, bytes)?;
            Some(lhs + &rhs)
        }
        "parenthesized_expression" => node.named_child(0).and_then(|n| literal_string(n, bytes)),
        // TypeScript-specific wrappers around expressions. These nodes carry no
        // field names in the 0.23 grammars, so unwrap structurally: the
        // expression is the first named child — except `<T>expr`, where the
        // type comes first and the expression last.
        "as_expression" | "satisfies_expression" | "non_null_expression" => {
            node.named_child(0).and_then(|n| literal_string(n, bytes))
        }
        "type_assertion" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .and_then(|n| literal_string(n, bytes)),
        _ => None,
    }
}

/// Strip comment syntax and detect the `Translators:` tag.
fn comment_text_line(line: &str) -> &str {
    let t = line.trim_start();
    let t = t.strip_prefix("//").unwrap_or(t);
    let t = t.strip_prefix("/*").unwrap_or(t);
    let t = t.strip_suffix("*/").unwrap_or(t);
    t.trim()
}

/// True when the comment is the only thing on its line (indentation allowed,
/// plus the wrapping braces of the idiomatic JSX `{/* Translators: … */}`).
/// Trailing comments after code — `gettext("x"); // Translators: …` — are
/// ignored by xgettext, so we must not attach them either.
fn comment_is_standalone(source: &str, row: usize, col: usize) -> bool {
    source
        .lines()
        .nth(row)
        .and_then(|line| line.get(..col))
        .is_some_and(|prefix| matches!(prefix.trim(), "" | "{"))
}

/// Find the comment block immediately above `call_row` (blank lines break the
/// chain) that carries a `Translators:` tag. Returns verbatim (row, line) pairs
/// so the dummy file reproduces the comment on its original lines.
fn translator_comment_for(
    call_row: usize,
    comments: &[(usize, usize, usize, String)],
    source: &str,
) -> Option<Vec<(usize, String)>> {
    // Find the comment that ends exactly one row above the call.
    let mut chain: Vec<(usize, usize, usize, String)> = Vec::new();
    let mut want_end = call_row.saturating_sub(1);
    if call_row == 0 {
        return None;
    }
    loop {
        let found = comments.iter().find(|(s, c, e, _)| {
            *e == want_end && *s <= want_end && comment_is_standalone(source, *s, *c)
        });
        match found {
            Some(&(s, c, e, ref text)) => {
                chain.insert(0, (s, c, e, text.clone()));
                if s == 0 {
                    break;
                }
                want_end = s - 1;
            }
            None => break,
        }
    }
    if chain.is_empty() {
        return None;
    }
    let tagged = chain.iter().any(|(_, _, _, text)| {
        text.lines()
            .any(|l| comment_text_line(l).starts_with("Translators"))
    });
    if !tagged {
        return None;
    }
    let mut lines = Vec::new();
    for (s, _, e, text) in &chain {
        let raw_lines: Vec<&str> = text.split('\n').collect();
        let span = e - s + 1;
        debug_assert_eq!(raw_lines.len(), span, "comment line span mismatch");
        for (i, line) in raw_lines.iter().enumerate() {
            lines.push((s + i, line.to_string()));
        }
    }
    Some(lines)
}

/// Render the dummy file: blank-line padded so every call (and its comment)
/// lands on its original line number.
pub fn render(extraction: &Extraction) -> String {
    let mut rows: Vec<String> = Vec::new();
    let ensure = |n: usize, rows: &mut Vec<String>| {
        while rows.len() <= n {
            rows.push(String::new());
        }
    };

    for call in &extraction.calls {
        ensure(call.row, &mut rows);

        // Place comment lines at their original rows. A collision means two
        // calls share one source line: xgettext attaches the single comment to
        // both msgids through that shared line, so re-emitting it would only
        // duplicate the comment — drop it instead.
        let comment_fits = call.comment.iter().all(|(row, _)| {
            ensure(*row, &mut rows);
            rows.get(*row).is_none_or(|r| r.is_empty())
        });
        if comment_fits {
            for (row, line) in &call.comment {
                ensure(*row, &mut rows);
                rows[*row] = line.clone();
            }
        }

        if rows[call.row].is_empty() {
            rows[call.row] = call.code.clone();
        } else {
            // Two calls share one source line.
            rows[call.row].push(' ');
            rows[call.row].push_str(&call.code);
        }
    }

    let mut out = rows.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Highest row the extraction writes to (used by tests).
#[cfg(test)]
pub fn max_row(extraction: &Extraction) -> Option<usize> {
    extraction.calls.iter().map(|c| c.row).max()
}

/// Check whether `output` is safe to wipe: not empty, not the source itself,
/// not an ancestor of source.
pub fn output_is_safe(source: &Path, output: &Path) -> bool {
    let src = std::fs::canonicalize(source).ok();
    let out = std::fs::canonicalize(output).ok();
    match (src, out) {
        (Some(s), Some(o)) => s != o && !s.starts_with(&o),
        _ => output.components().count() > 0,
    }
}
