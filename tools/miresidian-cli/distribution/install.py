#!/usr/bin/env python3
"""一次下载并安装完整工具套件；也可用 --archive 安装已下载的套件。Python 3.11+。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import sys
import tarfile
import tempfile
import urllib.request
import zipfile

REPOSITORY = "https://github.com/MyMires/miresidian"
REQUIRED = {"cargo-miresidian", "cargo-mires", "cargo-miresidian-pack", "cargo-miresidian-testflow-windows"}


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def fetch(url, target):
    request = urllib.request.Request(url, headers={"User-Agent": "miresidian-tools-installer"})
    with urllib.request.urlopen(request, timeout=60) as response, target.open("wb") as stream:
        shutil.copyfileobj(response, stream)


def latest():
    versions = []
    page = 1
    while True:
        request = urllib.request.Request(
            f"https://api.github.com/repos/MyMires/miresidian/releases?per_page=100&page={page}",
            headers={"User-Agent": "miresidian-tools-installer"})
        with urllib.request.urlopen(request, timeout=30) as response:
            releases = json.load(response)
        for release in releases:
            match = re.fullmatch(r"miresidian_cli-v(\d+)\.(\d+)\.(\d+)", release["tag_name"])
            if match and not release["draft"] and not release["prerelease"]:
                versions.append(tuple(map(int, match.groups())))
        if len(releases) < 100:
            break
        page += 1
    if not versions:
        raise ValueError("尚无公开工具套件正式版；请先使用本地套件或源码安装")
    return ".".join(map(str, max(versions)))


def host():
    if platform.machine().lower() not in {"x86_64", "amd64"}:
        raise ValueError("当前套件仅支持 x64 宿主")
    system = platform.system()
    if system not in {"Windows", "Linux"}:
        raise ValueError(f"当前套件尚不支持 {system}")
    return system.lower() + "-x64"


def stage_archive(archive, stage, expected_host, expected_version=None):
    checksum = archive.with_name(archive.name + ".sha256").read_text(encoding="ascii").split()
    if not checksum or checksum[0] != digest(archive):
        raise ValueError("套件压缩包 SHA-256 不匹配")
    # 只接受根目录普通文件，不提取任意路径或链接。
    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as package:
            members = package.infolist()
            for member in members:
                if Path(member.filename).name != member.filename or "\\" in member.filename or member.is_dir() or (member.external_attr >> 16) & 0o170000 == 0o120000:
                    raise ValueError("套件中存在非根目录普通文件")
            if len({m.filename for m in members}) != len(members):
                raise ValueError("套件含重复文件")
            for member in members:
                (stage / member.filename).write_bytes(package.read(member))
    else:
        with tarfile.open(archive, "r:gz") as package:
            members = package.getmembers()
            for member in members:
                if not member.isfile() or Path(member.name).name != member.name or "\\" in member.name:
                    raise ValueError("套件中存在非根目录普通文件")
            if len({m.name for m in members}) != len(members):
                raise ValueError("套件含重复文件")
            for member in members:
                with package.extractfile(member) as source:
                    (stage / member.name).write_bytes(source.read())
    manifest = json.loads((stage / "manifest.json").read_text(encoding="utf-8"))
    if manifest["format_version"] != 1 or manifest["host"] != expected_host:
        raise ValueError("套件格式或宿主系统不匹配")
    if expected_version and manifest["version"] != expected_version:
        raise ValueError("套件版本不匹配")
    names = set()
    for tool in manifest["tools"]:
        for file in tool["files"]:
            name = file["name"]
            if name in names or not re.fullmatch(r"cargo-[a-z0-9-]+(?:\.exe)?", name):
                raise ValueError("无效或重复的程序名称")
            if digest(stage / name) != file["sha256"]:
                raise ValueError(f"{name} SHA-256 不匹配")
            names.add(name)
    suffix = ".exe" if expected_host.startswith("windows") else ""
    if not {name + suffix for name in REQUIRED}.issubset(names):
        raise ValueError("套件缺少必要工具")
    if {p.name for p in stage.iterdir()} != names | {"manifest.json"}:
        raise ValueError("套件含未登记文件")
    return manifest, names


def install(archive, directory, expected_host, expected_version=None):
    with tempfile.TemporaryDirectory(prefix="miresidian-install-") as temporary:
        stage = Path(temporary)
        manifest, names = stage_archive(archive, stage, expected_host, expected_version)
        directory.mkdir(parents=True, exist_ok=True)
        # 全部校验通过后才写安装目录；写入失败时恢复已经替换的文件。
        backups = {}
        written = []
        try:
            for name in sorted(names) + ["manifest.json"]:
                target = directory / name
                if target.exists():
                    backups[name] = target.read_bytes()
                pending = directory / (".miresidian-" + name + ".tmp")
                try:
                    shutil.copyfile(stage / name, pending)
                    pending.chmod(0o644 if name == "manifest.json" else 0o755)
                    os.replace(pending, target)
                    written.append(name)
                finally:
                    pending.unlink(missing_ok=True)
        except OSError:
            for name in reversed(written):
                target = directory / name
                if name in backups:
                    target.write_bytes(backups[name])
                else:
                    target.unlink(missing_ok=True)
            raise
    print(f"已安装工具套件 {manifest['version']} 到 {directory}")
    for tool in manifest["tools"]:
        print(f"  {tool['package']} {tool['version']}")
    print(f"请将此目录加入 PATH：{directory}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", default="latest", help="套件版本，默认最新公开正式版")
    parser.add_argument("--archive", type=Path, help="本地压缩包，旁边须有同名 .sha256")
    parser.add_argument("--install-dir", type=Path, default=Path.home() / ".miresidian" / "bin")
    args = parser.parse_args()
    current_host = host()
    if args.archive:
        install(args.archive, args.install_dir, current_host, None if args.version == "latest" else args.version)
        return
    version = latest() if args.version == "latest" else args.version
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError("套件版本须为 X.Y.Z")
    extension = ".zip" if current_host.startswith("windows") else ".tar.gz"
    name = f"miresidian-tools-{version}-{current_host}{extension}"
    base = f"{REPOSITORY}/releases/download/miresidian_cli-v{version}"
    with tempfile.TemporaryDirectory(prefix="miresidian-download-") as temporary:
        archive = Path(temporary) / name
        fetch(f"{base}/{name}", archive)
        fetch(f"{base}/{name}.sha256", archive.with_name(name + ".sha256"))
        install(archive, args.install_dir, current_host, version)


if __name__ == "__main__":
    if sys.version_info < (3, 11):
        raise SystemExit("安装器需要 Python 3.11+")
    try:
        main()
    except (OSError, ValueError, KeyError, tarfile.TarError, zipfile.BadZipFile) as error:
        raise SystemExit(f"安装失败：{error}")
