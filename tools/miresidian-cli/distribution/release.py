#!/usr/bin/env python3
"""开发者批次发布：后端先于 CLI；默认 dry-run，显式 --publish 才上传 crates.io。"""
import argparse
from pathlib import Path
import subprocess
import tomllib
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[3]
catalog = tomllib.loads((ROOT / "tools/miresidian-cli/toolset.toml").read_text(encoding="utf-8"))


def exists(package, version):
    request = urllib.request.Request(
        f"https://crates.io/api/v1/crates/{package}/{version}",
        headers={"User-Agent": "miresidian-toolchain-release"})
    try:
        with urllib.request.urlopen(request, timeout=30):
            return True
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--publish", action="store_true")
    args = parser.parse_args()
    if subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip():
        raise ValueError("发布前须提交并验证版本标签，工作区必须干净")
    tools = sorted(catalog["tools"], key=lambda t: t["package"] == "miresidian_cli")
    for tool in tools:
        manifest = tomllib.loads((ROOT / tool["path"] / "Cargo.toml").read_text(encoding="utf-8"))
        if manifest["package"]["version"] != tool["version"]:
            raise ValueError(f"{tool['package']} 清单版本不一致")
        tag = f"{tool['package']}-v{tool['version']}"
        subprocess.run(["git", "rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        if subprocess.check_output(["git", "diff", tag, "HEAD", "--", tool["path"]], cwd=ROOT):
            raise ValueError(f"{tool['package']} 自版本标签后发生变化，必须独立升版")
        if exists(tool["package"], tool["version"]):
            print(f"已发布，跳过：{tool['package']} {tool['version']}", flush=True)
            continue
        command = ["cargo", "publish", "-p", tool["package"], "--locked"]
        if not args.publish:
            command.append("--dry-run")
        subprocess.run(command, cwd=ROOT, check=True)


if __name__ == "__main__":
    main()
