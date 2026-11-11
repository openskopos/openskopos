#!/bin/sh
# Builds the WebAssembly module of @openskopos/core into npm/core/dist.
#
# Needs cargo with the wasm32-unknown-unknown target. The script uses the
# wasm-bindgen CLI of the same version as the wasm-bindgen crate in
# Cargo.lock. If that version is not on the PATH, the script downloads it
# from the wasm-bindgen releases on GitHub into the cargo target folder.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
target_dir=${CARGO_TARGET_DIR:-$root/target}

version=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print; exit }' Cargo.lock)
if [ -z "$version" ]; then
  echo "build.sh: Cargo.lock has no wasm-bindgen" >&2
  exit 1
fi

bindgen=wasm-bindgen
if ! command -v wasm-bindgen > /dev/null 2>&1 ||
  [ "$(wasm-bindgen --version)" != "wasm-bindgen $version" ]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=x86_64-unknown-linux-musl ;;
    Linux-aarch64 | Linux-arm64) platform=aarch64-unknown-linux-musl ;;
    Darwin-x86_64) platform=x86_64-apple-darwin ;;
    Darwin-arm64) platform=aarch64-apple-darwin ;;
    *)
      echo "build.sh: install wasm-bindgen-cli $version: cargo install wasm-bindgen-cli --version $version --locked" >&2
      exit 1
      ;;
  esac
  name=wasm-bindgen-$version-$platform
  dir=$target_dir/wasm-bindgen
  if [ ! -x "$dir/$name/wasm-bindgen" ]; then
    mkdir -p "$dir"
    url=https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$version/$name.tar.gz
    echo "build.sh: downloading $url" >&2
    curl -fsSL "$url" -o "$dir/$name.tar.gz"
    curl -fsSL "$url.sha256sum" -o "$dir/$name.tar.gz.sha256sum"
    expected=$(cut -d ' ' -f 1 < "$dir/$name.tar.gz.sha256sum")
    if command -v sha256sum > /dev/null 2>&1; then
      actual=$(sha256sum "$dir/$name.tar.gz" | cut -d ' ' -f 1)
    else
      actual=$(shasum -a 256 "$dir/$name.tar.gz" | cut -d ' ' -f 1)
    fi
    if [ "$expected" != "$actual" ]; then
      echo "build.sh: the checksum of $name.tar.gz is wrong" >&2
      exit 1
    fi
    tar -xzf "$dir/$name.tar.gz" -C "$dir"
  fi
  bindgen=$dir/$name/wasm-bindgen
fi

# getrandom needs this flag for wasm32-unknown-unknown. .cargo/config.toml
# sets it, but cargo ignores that file's rustflags if RUSTFLAGS is set.
if [ -n "${RUSTFLAGS:-}" ]; then
  RUSTFLAGS="$RUSTFLAGS --cfg getrandom_backend=\"wasm_js\""
  export RUSTFLAGS
fi

cargo build -p skopos-wasm --target wasm32-unknown-unknown --profile wasm --locked
rm -rf npm/core/dist
# The name section holds the names of the Rust functions, about a third of
# the file. The package does not need it.
"$bindgen" --target web --out-dir npm/core/dist --out-name skopos \
  --remove-name-section --remove-producers-section \
  "$target_dir/wasm32-unknown-unknown/wasm/skopos_wasm.wasm"
echo "build.sh: built npm/core/dist" >&2
