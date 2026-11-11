#!/bin/sh
set -eu

fail() { printf 'already-tested: %s\n' "$*" >&2; exit 1; }

target() {
  os=$(uname -s)
  arch=$(uname -m)
  if [ "$os" = Darwin ] && [ "$(sysctl -n hw.optional.arm64 2>/dev/null || true)" = 1 ]; then
    arch=arm64
  fi
  case "$os" in
    MINGW*|MSYS*|CYGWIN*)
      arch=${PROCESSOR_ARCHITEW6432:-${PROCESSOR_ARCHITECTURE:-$arch}}
      if command -v pwsh >/dev/null 2>&1 &&
        native_arch=$(pwsh -NoProfile -NonInteractive -Command '[System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()'); then
        arch=$(printf '%s' "$native_arch" | tr -d '\r')
      fi ;;
  esac
  case "$arch" in
    x86_64|amd64|AMD64|X64) arch=x86_64 ;;
    arm64|aarch64|ARM64|Arm64) arch=aarch64 ;;
    *) fail "Unsupported architecture: $arch" ;;
  esac
  case "$os" in
    Linux) printf '%s-unknown-linux-musl\n' "$arch" ;;
    Darwin) printf '%s-apple-darwin\n' "$arch" ;;
    MINGW*|MSYS*|CYGWIN*) printf '%s-pc-windows-msvc\n' "$arch" ;;
    *) fail "Unsupported operating system: $os" ;;
  esac
}

version=${ALREADY_TESTED_VERSION:-latest}
bin_dir=${ALREADY_TESTED_BIN_DIR:-}
print_target=false
while [ "$#" -gt 0 ]; do
  case "$1" in
    --version|--bin-dir)
      [ "$#" -ge 2 ] && [ -n "$2" ] || fail "$1 requires a value"
      case "$1" in --version) version=$2 ;; --bin-dir) bin_dir=$2 ;; esac
      shift 2 ;;
    --print-target) print_target=true; shift ;;
    --help)
      printf '%s\n' 'Usage: install.sh [--version latest|vX.Y.Z] [--bin-dir PATH] [--print-target]'
      exit 0 ;;
    *) fail "Unknown option: $1" ;;
  esac
done
platform=$(target)
if [ "$print_target" = true ]; then printf '%s\n' "$platform"; exit 0; fi
if [ "$version" != latest ]; then
  version=${version#v}
  printf '%s\n' "$version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || fail 'Invalid version'
fi
[ -n "$bin_dir" ] || bin_dir="${HOME:?HOME is required}/.local/bin"
base=${ALREADY_TESTED_DOWNLOAD_BASE_URL:-}
if [ -z "$base" ]; then
  base=https://github.com/dblock/already-tested/releases
  if [ "$version" = latest ]; then base="$base/latest/download"; else base="$base/download/v$version"; fi
fi
case "$base" in https://*|file://*) ;; *) fail 'Download URL must use HTTPS (or file:// for local archives)' ;; esac
command -v git >/dev/null 2>&1 || fail 'Git is required'
command -v curl >/dev/null 2>&1 || fail 'curl is required for installation'
command -v tar >/dev/null 2>&1 || fail 'tar is required for installation'
temporary=$(mktemp -d)
staged=
cleanup() {
  [ -z "$staged" ] || rm -f -- "$staged"
  rm -f -- "$temporary/archive.tar.gz" "$temporary/checksum" "$temporary/already-tested" "$temporary/already-tested.exe"
  rmdir -- "$temporary"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
archive="already-tested-$platform.tar.gz"
curl --fail --silent --show-error --location --proto '=https,file' --proto-redir '=https,file' --tlsv1.2 \
  "${base%/}/$archive" -o "$temporary/archive.tar.gz"
curl --fail --silent --show-error --location --proto '=https,file' --proto-redir '=https,file' --tlsv1.2 \
  "${base%/}/$archive.sha256" -o "$temporary/checksum"
read -r expected name extra < "$temporary/checksum"
[ "$name" = "$archive" ] && [ -z "$extra" ] || fail 'Malformed checksum'
[ "$(wc -l < "$temporary/checksum" | tr -d ' ')" = 1 ] || fail 'Malformed checksum'
case "$expected" in ''|*[!0-9a-f]*) fail 'Malformed checksum' ;; esac
[ "${#expected}" = 64 ] || fail 'Malformed checksum'
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$temporary/archive.tar.gz")
elif command -v shasum >/dev/null 2>&1; then
  actual=$(shasum -a 256 "$temporary/archive.tar.gz")
else
  fail 'sha256sum or shasum is required for installation'
fi
actual=${actual%% *}
[ "$actual" = "$expected" ] || fail 'Archive checksum mismatch'
binary=already-tested
case "$platform" in *-windows-msvc) binary=already-tested.exe ;; esac
tar -xzf "$temporary/archive.tar.gz" -C "$temporary" -- "$binary"
[ -f "$temporary/$binary" ] && [ ! -L "$temporary/$binary" ] || fail 'Archive must contain a regular executable'
chmod 755 "$temporary/$binary"
installed_version=$("$temporary/$binary" --version)
printf '%s\n' "$installed_version" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || fail 'Invalid executable'
[ "$version" = latest ] || [ "$installed_version" = "$version" ] || fail 'Executable version mismatch'
mkdir -p -- "$bin_dir"
staged=$(mktemp "$bin_dir/.already-tested.XXXXXX")
cp -- "$temporary/$binary" "$staged"
chmod 755 "$staged"
[ ! -d "$bin_dir/$binary" ] || fail 'Executable path is a directory'
mv -f -- "$staged" "$bin_dir/$binary"
staged=
printf 'Installed already-tested %s to %s/%s\n' "$installed_version" "$bin_dir" "$binary"
