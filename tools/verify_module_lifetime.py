#!/usr/bin/env python3
"""Compile a private native host and prove module retention/unload/reload.

The fixture exposes test-only probe symbols, never production task ABI entries.
Build: cargo build --example module_lifetime --locked
Run: python3 tools/verify_module_lifetime.py target/debug/examples/libmodule_lifetime.so
On Windows use the matching module_lifetime.dll and a native MinGW C compiler.
"""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('library', type=Path)
    parser.add_argument('--cc', default=os.environ.get('CC', 'cc'))
    args = parser.parse_args()
    compiler = shutil.which(args.cc)
    if compiler is None:
        raise SystemExit(f'C compiler unavailable: {args.cc}')
    source = Path(__file__).parent / 'tests' / 'module_lifetime_host.c'
    with tempfile.TemporaryDirectory(prefix='miresidian-module-') as temporary:
        host = Path(temporary) / ('host.exe' if os.name == 'nt' else 'host')
        command = [compiler, '-std=c11', '-Wall', '-Wextra', '-Werror', str(source), '-o', str(host)]
        if os.name != 'nt':
            command += ['-D_GNU_SOURCE', '-pthread', '-ldl']
        else:
            command += ['-Wno-cast-function-type']
        subprocess.run(command, check=True)
        subprocess.run([str(host), str(args.library.resolve())], check=True, timeout=30)


if __name__ == '__main__':
    main()
