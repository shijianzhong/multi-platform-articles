#!/usr/bin/env sh
set -eu

REPO="${MPA_REPO:-shijianzhong/multi-platform-articles}"
VERSION="${MPA_VERSION:-}"
INSTALL_DIR="${MPA_INSTALL_DIR:-$HOME/.local/bin}"

if [ -z "$VERSION" ]; then
  echo "MPA_VERSION is required (e.g. v0.1.1)" >&2
  exit 2
fi

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"

case "$os" in
  linux) target_os="unknown-linux-gnu" ;;
  darwin) target_os="apple-darwin" ;;
  *) echo "unsupported os: $os" >&2; exit 2 ;;
esac

case "$arch" in
  x86_64|amd64) target_arch="x86_64" ;;
  arm64|aarch64) target_arch="aarch64" ;;
  *) echo "unsupported arch: $arch" >&2; exit 2 ;;
esac

target="${target_arch}-${target_os}"
name="mpa"
asset="${name}-${VERSION}-${target}.tar.gz"
url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"

mkdir -p "$INSTALL_DIR"
tmp="$(mktemp -d)"
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT

echo "Downloading $url"
curl -fsSL "$url" -o "$tmp/$asset"

tar -C "$tmp" -xzf "$tmp/$asset"
bin_path="$(find "$tmp" -type f -name 'mpa' -maxdepth 3 | head -n 1 || true)"
if [ -z "$bin_path" ]; then
  echo "mpa binary not found in archive" >&2
  exit 2
fi

install -m 0755 "$bin_path" "$INSTALL_DIR/mpa"
echo "Installed: $INSTALL_DIR/mpa"
echo "Run: mpa themes list"
echo "Config: run 'mpa' to open TUI and set WECHAT_APPID/WECHAT_SECRET"
