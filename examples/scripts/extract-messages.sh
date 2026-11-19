#!/usr/bin/env bash
# Extract translatable strings from BOTH sides of the demo project:
#
#   * Django templates/Python      → locale/<lang>/LC_MESSAGES/django.po
#   * React sources in frontend/src → locale/<lang>/LC_MESSAGES/djangojs.po
#
# GNU xgettext (which makemessages drives) cannot parse JSX, TypeScript or
# much of modern ES syntax — Django runs it with --language=JavaScript.
# Instead of fighting that parser, `i18n-mirror` first mirrors frontend/src
# into build/i18n-mirror: plain JS files containing only the normalized
# translation calls, each on its ORIGINAL line so the `#: …:LINE` references
# in the .po point at the real source. makemessages scans only the mirror,
# and the references are rewritten to frontend/src at the end.
#
# The tool is picked up from PATH if installed (pip install / uv add /
# cargo install); otherwise it is built from the crate at the repo root.
#
# Usage: scripts/extract-messages.sh [locale …]      (default locale: bg)
# Env:   I18N_KEEP_MIRROR=1  keep build/i18n-mirror for inspection
set -euo pipefail
cd "$(dirname "$0")/.."   # → examples/

LOCALES=("$@")
[ ${#LOCALES[@]} -gt 0 ] || LOCALES=(bg)

PY=.venv/bin/python
MIRROR_DIR=build/i18n-mirror

# ---------------------------------------------------------------- extractor --
if command -v i18n-mirror >/dev/null 2>&1; then
  MIRROR="i18n-mirror"
  echo "==> 1/4 using installed extractor: $(command -v i18n-mirror)"
else
  MIRROR="../target/release/i18n-mirror"
  echo "==> 1/4 no i18n-mirror on PATH; building from the crate at ../"
  cargo build --release --quiet --manifest-path ../Cargo.toml
fi

echo "==> 2/4 mirroring frontend/src → $MIRROR_DIR"
# The demo app wraps the catalog functions as t/nt/pt/npt (frontend/src/i18n.ts);
# teach the extractor those aliases alongside the defaults.
"$MIRROR" \
  --source frontend/src \
  --output "$MIRROR_DIR" \
  --functions gettext,gettext_noop,gettext_lazy,ngettext,ngettext_lazy,pgettext,pgettext_lazy,npgettext,npgettext_lazy,_,t,nt=ngettext,pt=pgettext,npt=npgettext

# ---------------------------------------------------------------- gettext ----
if ! command -v xgettext >/dev/null 2>&1; then
  GETTEXT_PREFIX="${GETTEXT_PREFIX:-$HOME/.local/opt/gettext-pkg}"
  if [ ! -x "$GETTEXT_PREFIX/tree/usr/bin/xgettext" ]; then
    echo "==> xgettext not found; bootstrapping gettext-tools (rootless)"
    scripts/bootstrap-gettext.sh "$GETTEXT_PREFIX"
  fi
  export PATH="$GETTEXT_PREFIX/tree/usr/bin:$PATH"
  export LD_LIBRARY_PATH="$GETTEXT_PREFIX/tree/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

# ----------------------------------------------------------- makemessages ---
# djangojs domain: scan ONLY the mirror. Everything else is excluded by name:
#   frontend/src      real sources (unparseable by xgettext — the whole point)
#   node_modules      dependencies (frontend/node_modules matches by basename)
#   frontend/dist     build output
# makemessages walks this directory (examples/), so the crate at the repo
# root is never visited.
echo "==> 3/4 makemessages: djangojs domain (mirror) + django domain (server side)"
for locale in "${LOCALES[@]}"; do
  $PY manage.py makemessages -d djangojs -l "$locale" \
    -e js,jsx,ts,tsx \
    -i 'frontend/src' -i 'node_modules' -i 'frontend/dist'
  $PY manage.py makemessages -d django -l "$locale" \
    -i 'node_modules' -i 'frontend/dist'
done

# --------------------------------------------------- rewrite po references ---
# xgettext records the path it was given (build/i18n-mirror/…). Point the
# references at the real sources so translators' editors jump to the code.
echo "==> 4/4 rewriting mirror references → frontend/src"
for po in locale/*/LC_MESSAGES/djangojs.po; do
  [ -e "$po" ] || continue
  sed -i.bak "s|#: $MIRROR_DIR/|#: frontend/src/|g" "$po" && rm -f "$po.bak"
  grep -c '^#: ' "$po" | xargs -I{} echo "    {} references in $po"
done

if [ "${I18N_KEEP_MIRROR:-0}" != "1" ]; then
  rm -rf "$MIRROR_DIR"
fi

echo "done. Translate the msgstr entries in locale/*/LC_MESSAGES/*.po, then run: make compile"
