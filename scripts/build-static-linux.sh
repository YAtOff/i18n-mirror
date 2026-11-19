#!/usr/bin/env bash
# Build and verify a fully static (musl) Linux binary of i18n-mirror using Docker.
#
# Output:
#   dist/i18n-mirror-linux-<arch>-musl
#
# Usage:
#   scripts/build-static-linux.sh [--arch x86_64|aarch64] [--skip-tests]
set -euo pipefail

cd "$(dirname "$0")/.."

ARCH=""
SKIP_TESTS=0

while [ $# -gt 0 ]; do
  case "$1" in
    --arch)
      [ $# -ge 2 ] || { echo "error: --arch requires a value (x86_64 or aarch64)" >&2; exit 2; }
      ARCH="$2"
      shift 2
      ;;
    --skip-tests)
      SKIP_TESTS=1
      shift
      ;;
    -h|--help)
      sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      exit 2
      ;;
  esac
done

if [ -z "$ARCH" ]; then
  case "$(uname -m)" in
    x86_64|amd64)  ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *)
      echo "error: unsupported host architecture: $(uname -m); pass --arch x86_64|aarch64" >&2
      exit 1
      ;;
  esac
fi

case "$ARCH" in
  x86_64)  DOCKER_PLATFORM="linux/amd64" ;;
  aarch64) DOCKER_PLATFORM="linux/arm64" ;;
  *)
    echo "error: unsupported --arch '$ARCH' (expected x86_64 or aarch64)" >&2
    exit 2
    ;;
esac

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is not installed or not on PATH" >&2
  exit 1
fi

mkdir -p dist

OUT_NAME="i18n-mirror-linux-${ARCH}-musl"
OUT_PATH="dist/${OUT_NAME}"
CARGO_VOL="i18n-mirror-cargo-${ARCH}"
TARGET_VOL="i18n-mirror-target-${ARCH}"

echo "==> 1/3 building static Linux (${ARCH}, musl) binary in Docker (${DOCKER_PLATFORM})..."
docker run --rm \
  --platform "${DOCKER_PLATFORM}" \
  -e "SKIP_TESTS=${SKIP_TESTS}" \
  -e "OUT_NAME=${OUT_NAME}" \
  -v "$(pwd):/src:ro" \
  -v "$(pwd)/dist:/out" \
  -v "${CARGO_VOL}:/usr/local/cargo/registry" \
  -v "${TARGET_VOL}:/target" \
  -w /src \
  rust:1-alpine \
  sh -euo pipefail -c '
    apk add --no-cache musl-dev gcc binutils file >/dev/null

    export RUSTFLAGS="-C target-feature=+crt-static"

    if [ "${SKIP_TESTS}" != "1" ]; then
      echo "--> running cargo test --all-features"
      cargo test --all-features --locked --target-dir /target
    fi

    echo "--> running cargo build --release"
    cargo build --release --locked --target-dir /target

    BIN="/target/release/i18n-mirror"

    echo "--> checking static linkage"
    file "${BIN}"
    if readelf -l "${BIN}" | grep -q "INTERP"; then
      echo "error: binary is dynamically linked (PT_INTERP header present)" >&2
      exit 1
    fi

    cp "${BIN}" "/out/${OUT_NAME}"
    chmod 0755 "/out/${OUT_NAME}"
  '

echo "==> 2/3 smoke-testing ${OUT_PATH} in a clean alpine container..."
docker run --rm \
  --platform "${DOCKER_PLATFORM}" \
  -v "$(pwd)/${OUT_PATH}:/usr/local/bin/i18n-mirror:ro" \
  -v "$(pwd)/tests/fixtures:/fixtures:ro" \
  alpine:latest \
  sh -euo pipefail -c '
    i18n-mirror --version
    i18n-mirror --source /fixtures --output /tmp/mirror-out --quiet
    test -f /tmp/mirror-out/modern.js
    test -f /tmp/mirror-out/tricky.tsx
    echo "--> fixture extraction OK ($(ls -1 /tmp/mirror-out | wc -l) files)"
  '

echo "==> 3/3 done: ${OUT_PATH} ($(ls -lh "${OUT_PATH}" | awk "{print \$5}"))"
