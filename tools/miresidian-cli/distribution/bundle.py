#!/usr/bin/env python3
"""构建独立工具并生成可校验的完整套件；不上传或发布。Python 3.11+。"""
import argparse
import hashlib
import json
import platform
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[3]
CATALOG = ROOT / "tools/miresidian-cli/toolset.toml"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def bundle(host, output, bin_dir=None):
    actual_host = platform.system().lower() + "-x64"
    if platform.machine().lower() not in {"x86_64", "amd64"} or host != actual_host:
        raise ValueError("只能为当前 x64 宿主构建并验证套件；其他宿主由相应 CI 构建")
    catalog = tomllib.loads(CATALOG.read_text(encoding="utf-8"))
    for tool in catalog["tools"]:
        actual = tomllib.loads((ROOT / tool["path"] / "Cargo.toml").read_text(encoding="utf-8"))
        if actual["package"]["version"] != tool["version"]:
            raise ValueError(f"套件版本与 {tool['package']} 的 Cargo.toml 不一致")
    if bin_dir is None:
        command = ["cargo", "build", "--release", "--locked"]
        for tool in catalog["tools"]:
            command += ["-p", tool["package"]]
        subprocess.run(command, cwd=ROOT, check=True)
        metadata = json.loads(subprocess.check_output(
            ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked"], cwd=ROOT))
        bin_dir = Path(metadata["target_directory"]) / "release"
    output.mkdir(parents=True, exist_ok=True)
    suffix = ".exe" if host == "windows-x64" else ""
    with tempfile.TemporaryDirectory(prefix="miresidian-bundle-") as temporary:
        stage = Path(temporary)
        tools = []
        for tool in catalog["tools"]:
            files = []
            for binary in tool["binaries"]:
                filename = binary + suffix
                source = bin_dir / filename
                version = subprocess.check_output([str(source), "--version"], text=True).strip().split()[-1]
                if version != tool["version"]:
                    raise ValueError(f"{filename} 实际版本 {version} 与套件清单不一致")
                shutil.copy2(source, stage / filename)
                files.append({"name": filename, "sha256": digest(source)})
            tools.append({"package": tool["package"], "version": tool["version"], "files": files})
        manifest = {"format_version": 1, "version": catalog["version"], "host": host, "tools": tools}
        (stage / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        base = f"miresidian-tools-{catalog['version']}-{host}"
        if suffix:
            archive = output / (base + ".zip")
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as package:
                for file in sorted(stage.iterdir()):
                    package.write(file, file.name)
        else:
            archive = output / (base + ".tar.gz")
            with tarfile.open(archive, "w:gz") as package:
                for file in sorted(stage.iterdir()):
                    package.add(file, arcname=file.name)
        archive.with_name(archive.name + ".sha256").write_text(f"{digest(archive)}  {archive.name}\n", encoding="ascii")
    print(archive)
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True, choices=["linux-x64", "windows-x64"])
    parser.add_argument("--output", type=Path, default=ROOT / "dist-tools")
    parser.add_argument("--bin-dir", type=Path, help="使用已构建的本机程序，省略时先执行 Cargo 构建")
    args = parser.parse_args()
    bundle(args.host, args.output, args.bin_dir.resolve() if args.bin_dir else None)
