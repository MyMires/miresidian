#!/usr/bin/env python3
"""提交前与 CI 共用的版本门禁，Python 3.11+，不访问网络。"""
import argparse
import datetime
import re
import subprocess
import sys
import tomllib
from pathlib import Path


class VersionError(Exception):
    pass


class Repository:
    def __init__(self, path, staged=False, ref=None):
        self.path = Path(path).resolve()
        self.staged = staged
        self.ref = ref

    def git(self, *args, optional=False):
        result = subprocess.run(
            ["git", "-C", str(self.path), *args], capture_output=True, text=True,
            encoding="utf-8", errors="strict",
        )
        if result.returncode and not optional:
            raise VersionError(result.stderr.strip() or f"git {' '.join(args)} 失败")
        return None if result.returncode else result.stdout

    def read(self, name, ref=None, optional=False):
        ref = ref or self.ref
        if ref or self.staged:
            value = self.git("show", f"{ref}:{name}" if ref else f":{name}", optional=True)
        else:
            file = self.path / name
            value = file.read_text(encoding="utf-8") if file.is_file() else None
        if value is None and not optional:
            raise VersionError(f"缺少 {name}（检查的是{'暂存区' if self.staged else '工作区'}）")
        return value

    def toml(self, name, ref=None):
        return tomllib.loads(self.read(name, ref))

    def files(self):
        if self.ref:
            return set(self.git("ls-tree", "-r", "--name-only", "-z", self.ref).split("\0")) - {""}
        result = self.git("ls-files", "--cached", "-z")
        if not self.staged:
            result += self.git("ls-files", "--others", "--exclude-standard", "-z")
        return set(result.split("\0")) - {""}


def version_tuple(value):
    if not re.fullmatch(r"\d+\.\d+\.\d+", value):
        raise VersionError(f"版本必须为 X.Y.Z：{value}")
    return tuple(map(int, value.split(".")))


def own_version(manifest):
    package = manifest["package"]
    value = package["version"]
    return manifest["workspace"]["package"]["version"] if isinstance(value, dict) else value


def exact_dependency(value):
    requirement = value if isinstance(value, str) else value.get("version", "")
    if not requirement.startswith("="):
        raise VersionError(f"第一方依赖必须使用精确版本 =X.Y.Z：{requirement}")
    version_tuple(requirement[1:])
    return requirement[1:]


def check_changelog(repo, name, version):
    text = repo.read(name)
    releases = re.findall(r"^## \[([^\]]+)\] - (\d{4}-\d{2}-\d{2})$", text, re.M)
    if not releases or releases[0][0] != version:
        raise VersionError(f"{name} 必须将本次变更归档到最新 ## [{version}] - YYYY-MM-DD，不能只写 Unreleased")
    datetime.date.fromisoformat(releases[0][1])
    section = text.split(f"## [{version}]", 1)[1].split("\n## ", 1)[0]
    if not re.search(r"^- \S", section, re.M):
        raise VersionError(f"{name} 的 {version} 缺少具体变更条目")
    unreleased = re.search(r"^## \[Unreleased\]\s*\n(.*?)(?=^## |\Z)", text, re.M | re.S)
    if unreleased and re.search(r"^- \S", unreleased[1], re.M):
        raise VersionError(f"{name} 仍有未归档的 Unreleased 条目")


def check_lock(repo, name, expected):
    packages = repo.toml(name).get("package", [])
    for package, version in expected.items():
        actual = [entry["version"] for entry in packages if entry["name"] == package]
        if actual != [version]:
            raise VersionError(f"{name} 的 {package} 应锁定 {version}，实际为 {actual}")


FRAMEWORK_CRATES = {"miresidian", "miresidian_abi", "miresidian_codec", "miresidian_lease", "miresidian_macro", "miresidian_instances"}
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"


def check_node_sources(repo, manifest, lock_name, prefix=""):
    # 新的 registry 约束不追溯改写旧规范下的历史提交。
    if repo.ref and "禁止 path" not in (repo.read("AGENTS.md", optional=True) or ""):
        return
    def walk(table):
        if not isinstance(table, dict):
            return
        for name, value in table.items():
            package = value.get("package", name.split(":", 1)[0]) if isinstance(value, dict) else name
            if package in FRAMEWORK_CRATES and isinstance(value, dict):
                if any(key in value for key in ("path", "git")) or value.get("registry", "crates-io") != "crates-io":
                    raise VersionError(f"子节点 {prefix} 的 {package} 必须依赖 crates.io 已发布版本，禁止 path/git 或其他 registry")
            if isinstance(value, dict):
                walk(value)
    walk(manifest)
    for config in (prefix + ".cargo/config.toml", prefix + ".cargo/config"):
        text = repo.read(config, optional=True)
        if text:
            data = tomllib.loads(text)
            if data.get("paths"):
                raise VersionError("子节点禁止 Cargo paths 本地覆盖")
            walk(data)
    for item in repo.toml(lock_name).get("package", []):
        if item["name"] in FRAMEWORK_CRATES:
            if item.get("source") != CRATES_IO_SOURCE or not re.fullmatch(r"[0-9a-f]{64}", item.get("checksum", "")):
                raise VersionError(f"{lock_name} 的 {item['name']} 必须由 Cargo 从 crates.io 解析，不能使用本地框架或缺少 registry 校验和")


def versioned_path(name):
    if name.startswith("tools/fixtures/"):
        return False  # 独立测试节点只提升自身版本，不触发库和工具无关升版。
    if name in {"Cargo.toml", "Cargo.lock", "README.md", ".cargo/config.toml"}:
        return True
    if name.startswith(("src/", "abi/", "codec/", "lease/", "macro/", ".github/workflows/")):
        return True
    if name.startswith("tools/"):
        return "/src/" in name or name.endswith(("Cargo.toml", "Cargo.lock", ".testflow", ".bat", ".ps1"))
    return name.startswith("docs/") and name not in {
        "docs/versioning.md", "docs/coding-standards.md", "docs/维护与发布.md",
    }


def check_base(repo, base, version, changed):
    old_text = repo.read("Cargo.toml", base, optional=True)
    if old_text is None:
        return
    old = own_version(tomllib.loads(old_text))
    current, previous = version_tuple(version), version_tuple(old)
    if current < previous:
        raise VersionError(f"版本不可回退：{old} -> {version}")
    manifest = repo.toml("Cargo.toml")
    members = manifest.get("workspace", {}).get("members", [])
    member_change = any(any(name.startswith(member + "/") for member in members) for name in changed)
    root_changed = [name for name in changed if not any(name.startswith(member + "/") for member in members)]
    if member_change:
        root_changed = [name for name in root_changed if name != "Cargo.lock"]
    if any(versioned_path(name) for name in root_changed) and current <= previous:
        raise VersionError(f"相关代码/构建/依赖发生变化，版本仍为 {old}；必须先升级版本并归档变更记录")


def check_member_versions(repo, base, manifest, paths, changed):
    if not base:
        return
    old_root = repo.toml("Cargo.toml", base)
    def actual(data, root):
        value = data["package"]["version"]
        return root["workspace"]["package"]["version"] if isinstance(value, dict) else value
    for path in paths[1:]:
        before = repo.read(path, base, optional=True)
        if before is None:
            continue
        old, new = tomllib.loads(before), repo.toml(path)
        old_version, new_version = actual(old, old_root), actual(new, manifest)
        splitting_unpublished_tool = (path.startswith("tools/")
            and old["package"]["version"] == {"workspace": True}
            and isinstance(new["package"]["version"], str)
            and not repo.git("rev-parse", "--verify", f"refs/tags/{new['package']['name']}-v{old_version}", optional=True))
        prefix = path.removesuffix("Cargo.toml")
        relevant = [name[len(prefix):] for name in changed if name.startswith(prefix)]
        old["package"].pop("version")
        new["package"].pop("version")
        if old == new:
            relevant = [name for name in relevant if name != "Cargo.toml"]
        impact = any(name.startswith("src/") or name in {"Cargo.toml", "README.md", "build.rs"} for name in relevant)
        if not splitting_unpublished_tool and (version_tuple(new_version) < version_tuple(old_version) or (impact and version_tuple(new_version) <= version_tuple(old_version))):
            raise VersionError(f"{path} 发生相关变更，必须独立提升该 crate 版本")
        if new_version != old_version:
            tag = f"{new['package']['name']}-v{new_version}"
            existing = repo.git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}", optional=True)
            if existing:
                head = repo.git("rev-parse", repo.ref or "HEAD").strip()
                committed = component_version(repo, new["package"]["name"], repo.ref or "HEAD")
                if repo.staged or existing.strip() != head or committed != new_version:
                    raise VersionError(f"{tag} 已存在，禁止复用或移动标签；请使用新版本")


def abi_signature(header):
    text = re.sub(r"/\*.*?\*/", "", header, flags=re.S)
    return {" ".join(re.findall(r"[A-Za-z_]\w*|\d+|\S", item)) for item in re.findall(r"^MIRESIDIAN_API\s+([^;]+);", text, re.M | re.S)}


def check_abi(repo, base):
    header = repo.read("abi/api/miresidian.h")
    source = repo.read("abi/src/lib.rs")
    metadata = repo.toml("abi/Cargo.toml")["package"]["metadata"]["miresidian"]["abi_version"]
    header_match = re.search(r"#define MIRESIDIAN_ABI_VERSION (\d+)", header)
    source_match = re.search(r"MIRESIDIAN_ABI_VERSION: u32 = (\d+)", source)
    if not header_match or not source_match or {metadata, int(header_match[1]), int(source_match[1])} != {metadata}:
        raise VersionError("ABI 元数据、Rust 常量和公共头文件版本不一致")
    if base:
        old = repo.read("abi/api/miresidian.h", base, optional=True)
        if old:
            match = re.search(r"#define MIRESIDIAN_ABI_VERSION (\d+)", old)
            if match and metadata < int(match[1]):
                raise VersionError("ABI 版本不可回退")
            if match and abi_signature(old) != abi_signature(header) and metadata <= int(match[1]):
                raise VersionError("C 导出签名发生变化，必须同步提升 ABI 版本")


def validate(repo, base):
    manifest = repo.toml("Cargo.toml")
    version = own_version(manifest)
    version_tuple(version)
    name = manifest["package"]["name"]
    check_changelog(repo, "CHANGELOG.md", version)
    changed = []
    if base:
        args = ["diff", "--name-only", "-z"]
        if repo.staged:
            args.append("--cached")
        if repo.ref:
            args += [base, repo.ref]
        else:
            args += [base]
        changed = repo.git(*args).split("\0")
        if not repo.staged and not repo.ref:
            changed += repo.git("ls-files", "--others", "--exclude-standard", "-z").split("\0")
        check_base(repo, base, version, changed)
    expected = {name: version}
    if "package" in manifest.get("workspace", {}):
        manifests = ["Cargo.toml"] + [f"{member}/Cargo.toml" for member in manifest["workspace"]["members"]]
        members = [repo.toml(path) for path in manifests]
        names = {member["package"]["name"] for member in members}
        check_member_versions(repo, base, manifest, manifests, changed)
        versions = {}
        for member in members:
            declared = member["package"]["version"]
            versions[member["package"]["name"]] = version if member is manifest else (
                manifest["workspace"]["package"]["version"] if declared == {"workspace": True} else declared)
        versions[name] = version
        for path, member in zip(manifests, members):
            member_name = member["package"]["name"]
            member_version = versions[member_name]
            version_tuple(member_version)
            if path != "Cargo.toml" and repo.read(path.removesuffix("Cargo.toml") + "CHANGELOG.md", optional=True):
                check_changelog(repo, path.removesuffix("Cargo.toml") + "CHANGELOG.md", member_version)
            expected[member_name] = member_version
            for dependency, specification in member.get("dependencies", {}).items():
                if dependency in names and exact_dependency(specification) != versions[dependency]:
                    raise VersionError(f"{path} 的 {dependency} 未精确依赖对应 crate 版本 {versions[dependency]}")
        check_abi(repo, base)
        fixtures = sorted(path for path in repo.files() if path.startswith("tools/fixtures/") and path.endswith("Cargo.toml"))
        for path in fixtures:
            fixture = repo.toml(path)
            fixture_framework = exact_dependency(fixture["dependencies"]["miresidian"])
            prefix = path.removesuffix("Cargo.toml")
            check_node_sources(repo, fixture, prefix + "Cargo.lock", prefix)
            fixture_version = fixture["package"]["version"]
            check_changelog(repo, prefix + "CHANGELOG.md", fixture_version)
            check_lock(repo, prefix + "Cargo.lock", {"miresidian": fixture_framework, fixture["package"]["name"]: fixture_version})
            if base and any(item.startswith(prefix) for item in changed):
                old = repo.read(path, base, optional=True)
                if old and version_tuple(fixture_version) <= version_tuple(tomllib.loads(old)["package"]["version"]):
                    raise VersionError(f"{path} 随框架重建时必须同步提升夹具自身版本")
    else:
        check_node_sources(repo, manifest, "Cargo.lock")
        dependency = manifest.get("dependencies", {}).get("miresidian")
        if dependency:
            framework = exact_dependency(dependency)
            expected["miresidian"] = framework
    check_lock(repo, "Cargo.lock", expected)
    tag = f"{name}-v{version}"
    existing = repo.git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}", optional=True)
    if existing and base and version != own_version(repo.toml("Cargo.toml", base)):
        head = repo.git("rev-parse", repo.ref or "HEAD").strip()
        committed_version = own_version(repo.toml("Cargo.toml", repo.ref or "HEAD"))
        if repo.staged or existing.strip() != head or committed_version != version:
            raise VersionError(f"{tag} 已存在，禁止复用或移动标签；请使用新版本")
    return version, tag


def component_version(repo, name, ref=None):
    root = repo.toml("Cargo.toml", ref)
    for path in ["Cargo.toml"] + [f"{member}/Cargo.toml" for member in root.get("workspace", {}).get("members", [])]:
        data = repo.toml(path, ref)
        if data["package"]["name"] == name:
            value = data["package"]["version"]
            return root["workspace"]["package"]["version"] if isinstance(value, dict) else value
    raise VersionError(f"工作区没有组件 {name}")


def check_message(repo, version, base, message):
    previous = repo.read("Cargo.toml", base, optional=True) if base else None
    if previous and own_version(tomllib.loads(previous)) == version:
        root = repo.toml("Cargo.toml")
        for member in root.get("workspace", {}).get("members", []):
            package = repo.toml(f"{member}/Cargo.toml")["package"]["name"]
            try:
                if component_version(repo, package) != component_version(repo, package, base):
                    return
            except VersionError:
                return
        if not re.search(r"版本影响\s*[:：]\s*无[，,；;：:\s]+\S", message):
            raise VersionError("未升版的纯测试/规范提交必须说明：版本影响：无；具体理由")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=".")
    parser.add_argument("--base", help="相关代码变化必须相对此 Git 引用提升版本")
    parser.add_argument("--component", help="独立组件名称；标签操作只作用于此组件")
    parser.add_argument("--staged", action="store_true", help="只读取暂存区，防止未暂存版本号掩盖漏改")
    parser.add_argument("--message-file", help="commit-msg 检查提交说明中的版本影响")
    parser.add_argument("--check-commits", action="store_true", help="逐提交检查基线之后已启用 AGENTS 规范的提交")
    parser.add_argument("--tag", action="store_true", help="提交后仅为版本升级提交创建不可移动的附注标签")
    parser.add_argument("--require-tag", action="store_true", help="确认当前版本标签指向 HEAD")
    args = parser.parse_args()
    if args.staged and (args.tag or args.require_tag or args.check_commits):
        parser.error("暂存区模式不能与标签或历史检查混用")
    repo = Repository(args.repo, args.staged, "HEAD" if args.tag or args.require_tag else None)
    base = args.base
    if base and set(base) == {"0"}:
        base = None
    try:
        version, tag = validate(repo, base)
        if args.component:
            version = component_version(repo, args.component)
            tag = f"{args.component}-v{version}"
        if args.message_file:
            check_message(repo, version, base, Path(args.message_file).read_text(encoding="utf-8"))
        if args.check_commits and base:
            for commit in repo.git("rev-list", "--reverse", f"{base}..HEAD").splitlines():
                if repo.read("AGENTS.md", commit, optional=True) is None:
                    continue  # 旧历史保留，从 P0 规则生效开始逐提交执行。
                parent = repo.git("rev-parse", "--verify", f"{commit}^", optional=True)
                snapshot = Repository(repo.path, ref=commit)
                previous = parent.strip() if parent else None
                commit_version, _ = validate(snapshot, previous)
                check_message(snapshot, commit_version, previous, repo.git("log", "-1", "--format=%B", commit))
        if args.tag:
            previous = repo.read("Cargo.toml", "HEAD^", optional=True)
            if not args.component and previous and own_version(tomllib.loads(previous)) == version:
                print("版本未变化，本次无需创建新标签")
                return
            existing = repo.git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}", optional=True)
            head = repo.git("rev-parse", "HEAD").strip()
            if existing and existing.strip() != head:
                raise VersionError(f"{tag} 已存在并指向其他提交，禁止移动")
            if not existing:
                repo.git("tag", "-a", tag, "-m", f"版本 {version}")
        if args.require_tag or args.tag:
            actual = repo.git("rev-parse", "--verify", f"refs/tags/{tag}^{{commit}}", optional=True)
            if not actual or actual.strip() != repo.git("rev-parse", "HEAD").strip():
                raise VersionError(f"缺少指向 HEAD 的版本标签 {tag}")
        print(f"版本门禁通过：{repo.path.name} {version}" + (f" / {tag}" if args.require_tag or args.tag else ""))
    except (VersionError, ValueError, KeyError) as error:
        print(f"版本门禁失败：{error}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
