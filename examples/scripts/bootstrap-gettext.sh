#!/usr/bin/env bash
# Bootstrap GNU gettext-tools WITHOUT root privileges.
#
# Django's makemessages shells out to xgettext/msguniq/msgmerge/msgattrib.
# On machines where you cannot `apt install gettext`, this script downloads
# the distribution's .deb packages (apt-get download needs no privileges)
# and extracts them into a local prefix with dpkg -x.
#
# Usage: scripts/bootstrap-gettext.sh [dest-dir]
#        (default: $HOME/.local/opt/gettext-pkg)
set -euo pipefail

DEST="${1:-$HOME/.local/opt/gettext-pkg}"

if command -v xgettext >/dev/null 2>&1; then
  echo "gettext-tools already available: $(command -v xgettext)"
  exit 0
fi

command -v apt-get >/dev/null 2>&1 || {
  echo "error: no xgettext on PATH and no apt-get to fetch it — install gettext-tools manually" >&2
  exit 1
}

mkdir -p "$DEST"
cd "$DEST"

echo "downloading gettext + missing runtime deps into $DEST …"
apt-get download gettext >/dev/null
for dep in $(apt-cache depends gettext | awk '/^ *Depends:/ {print $2}' | sort -u); do
  dpkg -s "$dep" >/dev/null 2>&1 || apt-get download "$dep" >/dev/null || echo "  (skip $dep: not downloadable, usually fine)"
done

mkdir -p tree
for deb in *.deb; do
  dpkg -x "$deb" tree
done

BIN="$DEST/tree/usr/bin"
export LD_LIBRARY_PATH="$DEST/tree/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
if ! "$BIN/xgettext" --version >/dev/null 2>&1; then
  echo "error: extracted xgettext does not run" >&2
  exit 1
fi
echo "gettext-tools ready: $BIN (add it to PATH, see scripts/extract-messages.sh)"
