#!/usr/bin/env python3
"""开发验证：用 Cargo 将三个独立 crate 安装到隔离根目录，再验证统一入口。不是用户安装器。"""
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib

repository = Path(__file__).resolve().parents[3]
catalog = tomllib.loads((repository / "tools/miresidian-cli/toolset.toml").read_text(encoding="utf-8"))
with tempfile.TemporaryDirectory(prefix="miresidian-cargo-verification-") as temporary:
    root = Path(temporary)
    install_root = root / "cargo home with spaces"
    for tool in catalog["tools"]:
        subprocess.run(["cargo", "install", "--path", str(repository / tool["path"]), "--root", str(install_root), "--locked"], check=True)
    binary = install_root / "bin"
    suffix = ".exe" if os.name == "nt" else ""
    cli = binary / ("cargo-miresidian" + suffix)
    short = binary / ("cargo-mires" + suffix)
    for tool in catalog["tools"]:
        for name in tool["binaries"]:
            version = subprocess.check_output([str(binary / (name + suffix)), "--version"], text=True).strip().split()[-1]
            assert version == tool["version"], (name, version)
    for subcommand in ["new", "check", "build", "pack", "testflow", "install", "update", "use", "list"]:
        subprocess.run([str(cli), subcommand, "--help"], check=True)
    recorded = subprocess.check_output([str(short), "list", "--root", str(install_root)], text=True)
    for tool in catalog["tools"]:
        assert tool["package"] + " v" + tool["version"] in recorded
    assert (install_root / ".crates.toml").is_file()
    subprocess.run([str(cli), "new", str(root / "demo-node"), "--name", "Cargo演示", "--mires-version", "0.14.0"], check=True)
    project = root / "demo-node"
    subprocess.run(["cargo", "check", "--locked"], cwd=project, check=True)
    subprocess.run([str(short), "check"], cwd=project, check=True)
    subprocess.run([str(cli), "testflow", "--os", "windows", "tools/smoke.testflow"], cwd=project, check=True)
    assert (project / "tools/smoke.bat").is_file()
print("Cargo 原生安装记录、双入口和新项目调用通过；Windows BAT 执行由对应 CI 验证。")
