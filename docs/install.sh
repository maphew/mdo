#!/bin/sh
set -eu

repo="maphew/mdo"
base_url="https://github.com/${repo}/releases/latest/download"
install_dir="${MDO_INSTALL_DIR:-$HOME/.local/bin}"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "mdo install: missing required command: $1" >&2
    exit 1
  fi
}

download() {
  url="$1"
  file="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$file"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$file" "$url"
  else
    echo "mdo install: missing curl or wget" >&2
    exit 1
  fi
}

verify_checksum() {
  asset="$1"

  if command -v sha256sum >/dev/null 2>&1; then
    grep "  ${asset}\$" SHA256SUMS | sha256sum -c -
  elif command -v shasum >/dev/null 2>&1; then
    grep "  ${asset}\$" SHA256SUMS | shasum -a 256 -c -
  else
    echo "mdo install: missing sha256sum or shasum" >&2
    exit 1
  fi
}

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux)
    case "$arch" in
      x86_64|amd64) ;;
      *)
        echo "mdo install: Linux archive is currently available for x86_64 only" >&2
        exit 1
        ;;
    esac
    asset="mdo-x86_64-unknown-linux-gnu.tar.gz"
    directory="mdo-x86_64-unknown-linux-gnu"
    bins="mdo mdo-open mdo-setup"
    ;;
  Darwin)
    asset="mdo-universal-apple-darwin.tar.gz"
    directory="mdo-universal-apple-darwin"
    bins="mdo mdo-open"
    ;;
  *)
    echo "mdo install: unsupported OS: $os" >&2
    exit 1
    ;;
esac

need tar
need grep
need install

tmp="$(mktemp -d 2>/dev/null || mktemp -d -t mdo-install)"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

cd "$tmp"
download "${base_url}/${asset}" "$asset"
download "${base_url}/SHA256SUMS" "SHA256SUMS"
verify_checksum "$asset"

mkdir -p "$install_dir"
tar -xzf "$asset"

for bin in $bins; do
  install -m 0755 "${directory}/${bin}" "${install_dir}/${bin}"
done

echo "mdo installed to ${install_dir}"
"${install_dir}/mdo" --version
