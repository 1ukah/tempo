#!/bin/sh
# Install tempo-cli for the current user on macOS and Linux.
# Copies tempo-cli to ~/.local/bin and adds that folder to your PATH.
set -eu

dir=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
src="$dir/tempo-cli"
bin="$HOME/.local/bin"

if [ ! -f "$src" ]; then
  echo "error: tempo-cli is not in the same folder as install.sh." >&2
  echo "Extract the whole archive, then run install.sh again." >&2
  exit 1
fi

mkdir -p "$bin"
# Copy to a temporary name first, so that a running tempo-cli does not block the update.
cp "$src" "$bin/.tempo-cli.tmp"
chmod 755 "$bin/.tempo-cli.tmp"
mv -f "$bin/.tempo-cli.tmp" "$bin/tempo-cli"

if [ "$(uname -s)" = Darwin ]; then
  # Files from the internet are blocked by macOS until this mark is removed.
  xattr -d com.apple.quarantine "$bin/tempo-cli" 2>/dev/null || true
fi

echo "Installed tempo-cli to $bin"

case ":$PATH:" in
  *":$bin:"*)
    echo "Done. Type: tempo-cli --help"
    exit 0
    ;;
esac

marker="# Added by the tempo installer"
case "$(basename -- "${SHELL:-sh}")" in
  zsh)
    rc="${ZDOTDIR:-$HOME}/.zshrc"
    line="export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
  bash)
    if [ "$(uname -s)" = Darwin ]; then rc="$HOME/.bash_profile"; else rc="$HOME/.bashrc"; fi
    line="export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
  fish)
    rc="$HOME/.config/fish/conf.d/tempo.fish"
    mkdir -p "$(dirname -- "$rc")"
    line="fish_add_path \"\$HOME/.local/bin\""
    ;;
  *)
    rc="$HOME/.profile"
    line="export PATH=\"\$HOME/.local/bin:\$PATH\""
    ;;
esac

if ! grep -qF "$marker" "$rc" 2>/dev/null; then
  printf '\n%s\n%s\n' "$marker" "$line" >> "$rc"
  echo "Added $bin to your PATH in $rc"
fi

echo "Done. Close this terminal and open a new one. Then type: tempo-cli --help"
