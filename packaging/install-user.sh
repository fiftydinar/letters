#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="${ROOT}/target/release/letters"
BIN_DIR="${HOME}/.local/bin"
APPS_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"

if [[ ! -x "${BIN_SRC}" ]]; then
    echo "Release binary not found. Build it first with: cargo build --release" >&2
    exit 1
fi

install -Dm755 "${BIN_SRC}" "${BIN_DIR}/letters"
install -Dm644 "${ROOT}/packaging/letters.desktop" "${APPS_DIR}/letters.desktop"
install -Dm644 "${ROOT}/packaging/letters.svg" "${ICON_DIR}/letters.svg"

update-desktop-database "${APPS_DIR}" >/dev/null 2>&1 || true
gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true

echo "Letters installed."
echo "Binary: ${BIN_DIR}/letters"
echo "Desktop entry: ${APPS_DIR}/letters.desktop"
echo "Launch it with: letters"
