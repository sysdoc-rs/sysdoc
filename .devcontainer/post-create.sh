#!/usr/bin/env bash
#
# Provisions the sysdoc dev container: the `just` command runner plus the cargo
# subcommands that `just check` needs. Re-running it is safe.

set -euo pipefail

CARGO_BIN="${CARGO_HOME:-/usr/local/cargo}/bin"

# The crate registry is a fresh Docker volume on first create and is owned by
# root; hand it to the container user so cargo can write to it.
if [ ! -w "${CARGO_HOME:-/usr/local/cargo}/registry" ]; then
    sudo chown -R "$(id -u):$(id -g)" "${CARGO_HOME:-/usr/local/cargo}/registry"
fi

echo "==> Installing system packages"
sudo apt-get update -qq
# unzip + curl are used by scripts/validate-docx.sh to fetch the OOXML validator.
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
    unzip curl

# cargo-binstall fetches prebuilt binaries, which takes seconds instead of the
# several minutes `cargo install` spends compiling cargo-deny from source.
install_binstall() {
    local target
    case "$(uname -m)" in
        x86_64) target="x86_64-unknown-linux-musl" ;;
        aarch64 | arm64) target="aarch64-unknown-linux-musl" ;;
        *) return 1 ;;
    esac

    local url="https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-${target}.tgz"
    echo "==> Installing cargo-binstall from ${url}"
    curl --proto '=https' --tlsv1.2 -fsSL "$url" | tar -xzf - -C "$CARGO_BIN" cargo-binstall
}

if ! command -v cargo-binstall >/dev/null 2>&1; then
    if ! install_binstall; then
        echo "==> Prebuilt cargo-binstall unavailable, building from source"
        cargo install cargo-binstall --locked
    fi
fi

echo "==> Installing just, cargo-deny and cargo-audit"
cargo binstall --no-confirm just cargo-deny cargo-audit

echo ""
echo "Dev container ready. Run 'just' to list the available tasks."
