use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use i18n_mirror::{render, Config, Kind};
use walkdir::WalkDir;

const USAGE: &str = "\
i18n-mirror — mirror gettext-style calls from modern JS/TS/JSX sources
into xgettext-safe dummy files, preserving file names and line numbers.

USAGE:
    i18n-mirror [OPTIONS]

OPTIONS:
    -s, --source <DIR>       Source directory to scan   [default: frontend/src]
    -o, --output <DIR>       Dummy mirror directory     [default: build/i18n-mirror]
    -f, --functions <LIST>   Comma-separated function names or name=kind entries.
                             kind: gettext | ngettext | pgettext | npgettext
                             [default: gettext,gettext_noop,gettext_lazy,_,t,
                                        ngettext,ngettext_lazy,pgettext,
                                        pgettext_lazy,npgettext,npgettext_lazy]
    -e, --extensions <LIST>  Comma-separated extensions [default: js,mjs,cjs,jsx,ts,mts,cts,tsx]
    --deny-warnings          Exit with status 1 if any warning was emitted
    -q, --quiet              Only print errors
    -V, --version            Print version
    -h, --help               Show this help
";

struct Args {
    source: PathBuf,
    output: PathBuf,
    functions: Option<String>,
    extensions: Option<String>,
    deny_warnings: bool,
    quiet: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        source: PathBuf::from("frontend/src"),
        output: PathBuf::from("build/i18n-mirror"),
        functions: None,
        extensions: None,
        deny_warnings: false,
        quiet: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-s" | "--source" => {
                args.source = PathBuf::from(it.next().ok_or("missing value for --source")?)
            }
            "-o" | "--output" => {
                args.output = PathBuf::from(it.next().ok_or("missing value for --output")?)
            }
            "-f" | "--functions" => {
                args.functions = Some(it.next().ok_or("missing value for --functions")?)
            }
            "-e" | "--extensions" => {
                args.extensions = Some(it.next().ok_or("missing value for --extensions")?)
            }
            "--deny-warnings" => args.deny_warnings = true,
            "-q" | "--quiet" => args.quiet = true,
            "-V" | "--version" => {
                println!("i18n-mirror {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other} (see --help)")),
        }
    }
    Ok(args)
}

fn build_config(functions: Option<&str>, extensions: Option<&str>) -> Result<Config, String> {
    let mut config = Config::default();

    if let Some(list) = functions {
        let mut map = HashMap::new();
        for entry in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (name, kind) = match entry.split_once('=') {
                Some((n, k)) => {
                    let kind = Kind::parse(k).ok_or_else(|| {
                        format!("unknown kind {k:?} in --functions entry {entry:?}")
                    })?;
                    (n.trim().to_string(), kind)
                }
                None => {
                    // Bare name: look it up in the default table so aliases
                    // keep their gettext shape.
                    let kind = i18n_mirror::DEFAULT_FUNCTIONS
                        .iter()
                        .find(|(n, _)| *n == entry)
                        .map(|(_, k)| *k)
                        .ok_or_else(|| format!(
                            "unknown function {entry:?}; use name=kind with kind in gettext|ngettext|pgettext|npgettext"
                        ))?;
                    (entry.to_string(), kind)
                }
            };
            if name.is_empty() {
                return Err(format!(
                    "empty function name in --functions entry {entry:?}"
                ));
            }
            map.insert(name, kind);
        }
        config.functions = map;
    }

    if let Some(list) = extensions {
        let set: HashSet<String> = list
            .split(',')
            .map(str::trim)
            .map(|s| s.trim_start_matches('.').to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if set.is_empty() {
            return Err("--extensions list is empty".into());
        }
        config.extensions = set;
    }
    Ok(config)
}

/// Directories never worth scanning.
const SKIPPED_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "out",
    "coverage",
    ".git",
    ".cache",
    ".venv",
];

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let config = match build_config(args.functions.as_deref(), args.extensions.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let source = &args.source;
    let output = &args.output;
    if !source.is_dir() {
        eprintln!(
            "error: source directory {} does not exist",
            source.display()
        );
        return ExitCode::FAILURE;
    }
    if !i18n_mirror::output_is_safe(source, output) {
        eprintln!(
            "error: refusing to use {} as output: it is the source or an ancestor of it",
            output.display()
        );
        return ExitCode::FAILURE;
    }

    // Fresh mirror tree every run so renamed/deleted sources do not linger.
    match std::fs::remove_dir_all(output) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("error: cannot clear {}: {e}", output.display());
            return ExitCode::FAILURE;
        }
    }

    let mut total_calls = 0usize;
    let mut total_files = 0usize;
    let mut total_warnings = 0usize;

    let walker = WalkDir::new(source).into_iter().filter_entry(|e| {
        let p = e.path();
        if e.depth() == 0 {
            return true;
        }
        if p.is_dir() && p != output {
            if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                if SKIPPED_DIRS.contains(&name) {
                    return false;
                }
            }
        }
        p != output
    });

    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
        else {
            continue;
        };
        if !config.extensions.contains(&ext) {
            continue;
        }
        let meta = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("warning: {}: {e}", path.display());
                total_warnings += 1;
                continue;
            }
        };
        if meta.len() > 2_000_000 {
            eprintln!(
                "warning: {}: file larger than 2 MB (generated bundle?) — skipped",
                path.display()
            );
            total_warnings += 1;
            continue;
        }
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("warning: {}: {e}", path.display());
                total_warnings += 1;
                continue;
            }
        };
        let source_text = match String::from_utf8(bytes) {
            Ok(s) => {
                // Strip a UTF-8 BOM if present.
                s.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(s)
            }
            Err(_) => {
                eprintln!("warning: {}: not valid UTF-8 — skipped", path.display());
                total_warnings += 1;
                continue;
            }
        };

        let extraction = match i18n_mirror::extract(&source_text, &ext, &config) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("warning: {}: {e}", path.display());
                total_warnings += 1;
                continue;
            }
        };

        for w in &extraction.warnings {
            eprintln!("warning: {}: {w}", path.display());
        }
        total_warnings += extraction.warnings.len();
        if extraction.had_parse_errors && !extraction.is_empty() {
            eprintln!(
                "warning: {}: file contains syntax errors; extraction may be incomplete",
                path.display()
            );
            total_warnings += 1;
        }

        if extraction.is_empty() {
            continue;
        }

        let rel = path.strip_prefix(source).unwrap_or(path);
        let dest = output.join(rel);
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("error: cannot create {}: {e}", parent.display());
                return ExitCode::FAILURE;
            }
        }
        if let Err(e) = std::fs::write(&dest, render(&extraction)) {
            eprintln!("error: cannot write {}: {e}", dest.display());
            return ExitCode::FAILURE;
        }

        total_calls += extraction.calls.len();
        total_files += 1;
        if !args.quiet {
            println!(
                "  {} → {} ({} call{})",
                path.display(),
                dest.display(),
                extraction.calls.len(),
                if extraction.calls.len() == 1 { "" } else { "s" }
            );
        }
    }

    if !args.quiet {
        println!(
            "i18n-mirror: {} message call{} from {} file{} → {}",
            total_calls,
            if total_calls == 1 { "" } else { "s" },
            total_files,
            if total_files == 1 { "" } else { "s" },
            output.display()
        );
    }
    if total_warnings > 0 {
        eprintln!("i18n-mirror: {total_warnings} warning(s)");
        if args.deny_warnings {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

// Keep Path import used even if config paths change.
#[allow(dead_code)]
fn _unused(_p: &Path) {}
