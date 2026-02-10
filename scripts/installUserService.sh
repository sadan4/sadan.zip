#!/usr/bin/env bash
set -euo pipefail
mkdir -p "$HOME/.config/systemd/user/"

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cp "$SCRIPT_DIR/../browser.service" "$HOME/.config/systemd/user/"