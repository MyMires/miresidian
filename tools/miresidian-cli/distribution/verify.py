#!/usr/bin/env python3
"""从本地套件安装到临时目录，并以真实独立程序验证新建、检查和 BAT 生成。"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import install

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--archive", required=True, type=Path)
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix="miresidian-suite-verification-") as temporary:
    root = Path(temporary)
    binary = root / "install with spaces"
    install.install(args.archive.resolve(), binary, install.host())
    suffix = ".exe" if os.name == "nt" else ""
    cli = binary / ("cargo-miresidian" + suffix)
    short = binary / ("cargo-mires" + suffix)
    manifest = json.loads((binary / "manifest.json").read_text(encoding="utf-8"))
    for tool in manifest["tools"]:
        for file in tool["files"]:
            version = subprocess.check_output([str(binary / file["name"]), "--version"], text=True).strip().split()[-1]
            assert version == tool["version"], (file["name"], version)
    for subcommand in ["new", "check", "build", "pack", "testflow"]:
        subprocess.run([str(cli), subcommand, "--help"], check=True)
    subprocess.run([str(cli), "new", str(root / "demo-node"), "--name", "套件演示", "--mires-version", "0.14.0"], check=True)
    project = root / "demo-node"
    subprocess.run(["cargo", "check", "--locked"], cwd=project, check=True)
    subprocess.run([str(short), "check"], cwd=project, check=True)
    subprocess.run([str(cli), "testflow", "--os", "windows", "tools/smoke.testflow"], cwd=project, check=True)
    assert (project / "tools/smoke.bat").is_file()
print("整套安装与进程调用验证通过；未将 BAT 生成等同于 Windows 执行。")
