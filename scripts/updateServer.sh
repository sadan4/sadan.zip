#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

pushd "$SCRIPT_DIR/.."
if [[ -n "$(git status --porcelain)" ]]; then
    echo "Working directory is not clean. Please commit or stash changes before updating.";
    exit 1;
fi
git pull --no-edit
systemctl --user restart browser.service
popd

