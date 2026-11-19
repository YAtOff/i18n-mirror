# Example: Django + React translation pipeline

A complete, working integration of [`i18n-mirror`](../README.md) — the Rust
extractor at the repo root — into a real project:

- **Django 6.1** project (`demo/`, `shop/`) with `LocaleMiddleware`,
  `LOCALE_PATHS`, the `JavaScriptCatalog` view at `/jsi18n/`, and a
  server-rendered page consuming both translation domains.
- **Vite 8 + React 19 + TypeScript 7** app (`frontend/`) whose typed catalog
  aliases (`t`, `nt`, `pt`, `npt` in `frontend/src/i18n.ts`) map to the
  documented `jsi18n` functions.
- **Bulgarian translations** (`locale/bg/`) produced by this very pipeline —
  contexts, plural forms and `Translators:` comments intact.
- **End-to-end test** (`tests/verify_catalog.mjs`) that boots the server,
  downloads `/jsi18n/`, executes it like a browser, and asserts the
  translations.

## Quickstart

```console
# 1. Python side (Django) — pip is bootstrapped via get-pip.py if needed
$ make venv

# 2. Frontend deps
$ cd frontend && npm install && cd ..

# 3. Install the extractor (recommended), or let the scripts build it from ../
$ pip install i18n-mirror          # or: uv pip install i18n-mirror
                                   # or run it via: uvx i18n-mirror --version

# 4. Extract → translate → compile
$ make messages LOCALES=bg         # mirror → makemessages (both domains) → rewrite refs
$ $EDITOR locale/bg/LC_MESSAGES/*.po
$ make compile                     # compilemessages

# 5. Prove it works
$ make verify                      # boots server, asserts the live catalog (node)
$ make demo                        # runserver 127.0.0.1:8000
```

`make messages` uses the `i18n-mirror` binary from `PATH` when installed and
falls back to `cargo build` on the crate at the repository root.

## No root for gettext?

`scripts/bootstrap-gettext.sh` installs GNU gettext-tools **without
privileges**: it downloads the distribution's `.deb` packages with
`apt-get download` and extracts them locally with `dpkg -x`. The pipeline
calls it automatically when `xgettext` is missing.

## What to look at

| path | why it's interesting |
|---|---|
| `frontend/src/components/Cart.tsx` | `ngettext`/`pgettext`/`npgettext` + translator comments in JSX |
| `frontend/src/i18n.ts` | typed bindings to the documented catalog functions |
| `scripts/extract-messages.sh` | the full 4-step pipeline wiring |
| `locale/bg/LC_MESSAGES/djangojs.po` | output: exact `#: frontend/src/…:LINE` refs |
| `tests/verify_catalog.mjs` | runtime assertions on the served catalog |
