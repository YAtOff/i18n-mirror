# i18n-mirror

Extract translatable strings from a modern **React / TypeScript / JSX** frontend
into Django's standard gettext workflow — with **zero Node.js tooling in the
extraction step** and **exact source line numbers** in the `.po` files.

`django-admin makemessages -d djangojs` shells out to GNU
`xgettext --language=JavaScript`, a parser that predates modern JavaScript: it
chokes on JSX, TypeScript syntax, optional chaining and more. **i18n-mirror**
is a single Rust binary (tree-sitter with real JavaScript/TypeScript/TSX
grammars) that solves this with *source mirroring*: it generates a parallel
tree of dummy JS files that xgettext parses without difficulty, while
preserving the original file names, line numbers and `Translators:` comments —
so the `.po` references (`#: frontend/src/components/Cart.tsx:12`) point at the
real source.

```po
#. Translators: summary of the shopping cart; %(count)s is the item count
#: frontend/src/components/Cart.tsx:12
msgid "1 item"
msgid_plural "%(count)s items"
msgstr[0] "1 артикул"
msgstr[1] "%(count)s артикула"
```

Contexts (`msgctxt`), plural forms and translator comments all survive;
clicking a reference in Poedit or Weblate jumps to the real TSX line.

## Install

The recommended channel for Django teams is **PyPI** — the Rust binary is
packaged with [maturin](https://www.maturin.rs)' `bin` bindings (the same
mechanism Ruff uses), as ABI-independent wheels built by CI for Linux
(glibc + musl), macOS and Windows:

```console
$ pip install i18n-mirror          # into the project venv
$ uv add --dev i18n-mirror         # same wheels, as a dev dependency
```

No Rust toolchain is needed on the installing machine. From source instead:

```console
$ pip install .                    # builds via maturin (needs cargo)
$ cargo install --path .           # pure cargo route
```

## Running it

After `pip install` / `uv add`, the `i18n-mirror` console command lives in the
environment's `bin/` — the usual ways to reach it:

```console
# pip: activate the venv, or call it by path
$ source .venv/bin/activate && i18n-mirror --version
$ .venv/bin/i18n-mirror --version

# uv project (declared with `uv add --dev i18n-mirror`)
$ uv run i18n-mirror --source frontend/src --output build/i18n-mirror --functions _,t

# one-shot, no install and no project changes (great for CI)
$ uvx i18n-mirror --version
$ uvx --from /path/to/checkout i18n-mirror --version     # builds via maturin
```

Two properties that matter when wiring it into scripts and Makefiles:

- `uv run <script>` puts the environment's `bin/` on `PATH` for the child
  process, so a wrapper script can simply `command -v i18n-mirror` and use it
  (that is exactly how [`examples/scripts/extract-messages.sh`](examples/scripts/extract-messages.sh)
  picks the binary up — falling back to `cargo build` on the sibling crate).
- In CI, run the mirror step with `--deny-warnings` so dynamic or
  non-extractable msgids fail the build instead of silently dropping strings.

For the full Django integration (mirror → `makemessages` → reference rewrite →
compile → runtime verification) see [Hooking it into Django](#hooking-it-into-django)
and the working example under [`examples/`](examples/). The CLI in full:

```
i18n-mirror — mirror gettext-style calls from modern JS/TS/JSX sources
into xgettext-safe dummy files, preserving file names and line numbers.

    -s, --source <DIR>       Source directory to scan   [default: frontend/src]
    -o, --output <DIR>       Dummy mirror directory     [default: build/i18n-mirror]
    -f, --functions <LIST>   names or name=kind (gettext|ngettext|pgettext|npgettext)
    -e, --extensions <LIST>  [default: js,mjs,cjs,jsx,ts,mts,cts,tsx]
    --deny-warnings          exit 1 if any warning was emitted (CI mode)
```

What the parser handles:

- **All four gettext shapes** — `gettext`, `ngettext`, `pgettext`,
  `npgettext` — plus configurable aliases: `_`, `t`, `nt`, `pt`, your own
  (`--functions myT=gettext`), and member calls (`i18n.t(…)`,
  `window.ngettext(…)`, optional chains included).
- **String exactness** — both quote styles and template literals without
  substitutions are fully decoded and re-encoded, so msgids match the runtime
  values byte-for-byte. Concatenation is folded (`_("a " + "b")`), TS
  `as`/`satisfies` casts are unwrapped.
- **Comment semantics that match xgettext** — standalone `//`, `/* */` and JSX
  `{/* Translators: … */}` comments attach to the call directly below; trailing
  same-line comments do not; blank lines break the chain.
- **Line-grid output** — every dummy call and comment lands on its original
  line, so `#: file.tsx:LINE` is exact (two calls sharing a source line share a
  dummy line).
- **Honest failures** — dynamic msgids, i18next option objects, or templates
  with substitutions are skipped with a `file:line` warning instead of
  silently producing wrong msgids. `--deny-warnings` turns them into CI
  failures.

## Hooking it into Django

Wrap your `makemessages` invocation: mirror the sources, let makemessages scan
**only** the mirror, then rewrite the `#:` references back to the real paths.
The complete, working integration — Django project, React 19 + Vite 8 app,
Bulgarian translations and an end-to-end runtime test — lives in
[`examples/`](examples/):

```console
$ cd examples && make messages LOCALES=bg
```

A minimal version of the wiring:

```sh
i18n-mirror --source frontend/src --output build/i18n-mirror --functions _,t,nt=ngettext
python manage.py makemessages -d djangojs -l bg -e js,jsx,ts,tsx \
    -i 'frontend/src' -i 'node_modules' -i 'frontend/dist'
sed -i 's|#: build/i18n-mirror/|#: frontend/src/|g' locale/bg/LC_MESSAGES/djangojs.po
rm -rf build/i18n-mirror
```

(`makemessages` forces `--language=JavaScript` for the whole `djangojs`
domain, so the dummy `.tsx`/`.jsx` files are parsed as plain JS — that's fine,
they contain only normalized calls.)

At runtime the frontend uses the catalog Django already serves — no client
i18n framework — via the documented
[`JavaScriptCatalog`](https://docs.djangoproject.com/en/6.1/topics/i18n/translation/#translating-javascript-strings)
functions: `gettext`, `ngettext`, `pgettext`, `npgettext`, `interpolate`,
`get_format`, `pluralidx`, loaded with `<script src="/jsi18n/"></script>`.

## Testing

```console
$ cargo test                 # 16 tests: codecs, line-grid invariants (dummy
                             # output is re-parsed as JS to prove validity),
                             # comment semantics, CLI table
```

`tests/realworld/` holds **unmodified files from Outline, Excalidraw,
react-admin and Zulip** (see its `SOURCES.md`) — real modern syntax, zero
parse errors. The demo under `examples/` adds a Node end-to-end check that
boots Django, executes the served `/jsi18n/` script and asserts Bulgarian
translations, plural selection and `interpolate()` output.

## Releasing

Push a `v*` tag: [`.github/workflows/release.yml`](.github/workflows/release.yml)
runs the tests, builds wheels for linux (glibc + musl), macOS and Windows plus
an sdist, publishes to PyPI (trusted publishing) and attaches everything to
the GitHub Release. Publishing to crates.io additionally is one
`cargo publish` away.

## Notes & limits

- `.vue`/`.svelte` single-file components are not scanned (extend with `-e`).
- Object-literal i18n APIs (e.g. Zulip's `$t({defaultMessage: …})`) are
  deliberately out of scope: there is no sound mapping to gettext argument
  positions.
- The mirror step deletes and recreates its output directory each run; it
  never touches sources (and refuses unsafe output paths).
