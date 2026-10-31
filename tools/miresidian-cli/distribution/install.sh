#!/bin/sh
set -eu
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if command -v python3 >/dev/null 2>&1; then
    exec python3 "$script_dir/install.py" "$@"
fi
echo "需要 Python 3.11+；也可手动解压完整工具套件并将目录加入 PATH。" >&2
exit 1
