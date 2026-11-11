"""版本门禁的实际 Git 仓库回归，不访问网络。"""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECK = Path(__file__).resolve().parents[1] / "check_versions.py"


class VersionGateTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.repo = Path(self.directory.name)
        self.git("init", "-q")
        self.git("config", "user.name", "Version gate test")
        self.git("config", "user.email", "version-test@example.invalid")
        self.write("AGENTS.md", "# P0 版本门禁\n")
        self.release("0.1.0")
        self.write("src/lib.rs", "pub fn value() -> u32 { 1 }\n")
        self.commit()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.repo), *args], text=True).strip()

    def write(self, name, text):
        file = self.repo / name
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, encoding="utf-8")

    def release(self, version, locked=None):
        self.write("Cargo.toml", f'''[package]
name = "sample_node"
version = "{version}"
[dependencies]
miresidian = "=0.11.0"
''')
        self.write("Cargo.lock", f'''version = 4
[[package]]
name = "sample_node"
version = "{locked or version}"
[[package]]
name = "miresidian"
version = "0.11.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "0000000000000000000000000000000000000000000000000000000000000000"
''')
        self.write("CHANGELOG.md", f"# Changes\n\n## [Unreleased]\n\n## [{version}] - 2026-10-29\n\n- 修改设备行为。\n")

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "test: 创建版本门禁夹具")
        return self.git("rev-parse", "HEAD")

    def check(self, *args):
        return subprocess.run(
            [sys.executable, str(CHECK), "--repo", str(self.repo), *args],
            capture_output=True, text=True, encoding="utf-8",
        )

    def test_code_change_without_version_is_rejected(self):
        self.write("src/lib.rs", "pub fn value() -> u32 { 2 }\n")
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("必须先升级版本", result.stderr)

    def test_unstaged_version_cannot_hide_missing_staged_version(self):
        self.write("src/lib.rs", "pub fn value() -> u32 { 2 }\n")
        self.git("add", "src/lib.rs")
        self.release("0.2.0")
        self.assertNotEqual(self.check("--base", "HEAD", "--staged").returncode, 0)
        self.git("add", ".")
        self.assertEqual(self.check("--base", "HEAD", "--staged").returncode, 0)

    def test_stale_lock_and_unreleased_changes_are_rejected(self):
        self.release("0.2.0", locked="0.1.0")
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Cargo.lock", result.stderr)
        self.release("0.2.0")
        file = self.repo / "CHANGELOG.md"
        file.write_text(file.read_text().replace("## [Unreleased]", "## [Unreleased]\n\n- 遗漏归档。"), encoding="utf-8")
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unreleased", result.stderr)

    def test_tag_is_created_from_committed_version_and_never_moved(self):
        self.release("0.2.0")
        head = self.commit()
        self.release("0.3.0")  # 未提交版本不得影响提交后的标签。
        self.assertEqual(self.check("--tag").returncode, 0)
        self.assertEqual(self.git("rev-parse", "sample_node-v0.2.0^{commit}"), head)
        self.assertEqual(self.check("--require-tag").returncode, 0)
        self.release("0.2.0")
        self.write("docs/versioning.md", "# 规范说明\n")
        self.commit()
        self.assertEqual(self.check("--tag").returncode, 0)
        self.assertEqual(self.git("rev-parse", "sample_node-v0.2.0^{commit}"), head)

    def test_existing_tag_cannot_be_reused(self):
        self.git("tag", "sample_node-v0.2.0")
        self.release("0.2.0")
        self.git("add", ".")
        result = self.check("--base", "HEAD", "--staged")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("禁止复用", result.stderr)

    def test_ci_checks_each_commit_not_only_final_version(self):
        base = self.git("rev-parse", "HEAD")
        self.release("0.2.0")
        self.commit()
        self.write("src/lib.rs", "pub fn value() -> u32 { 3 }\n")
        self.commit()
        self.assertEqual(self.check("--base", base).returncode, 0)
        result = self.check("--base", base, "--check-commits")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("必须先升级版本", result.stderr)

    def test_document_only_commit_requires_explicit_impact_and_reason(self):
        self.write("docs/versioning.md", "# 更新版本管理规范\n")
        self.git("add", ".")
        self.write(".git/COMMIT_EDITMSG", "docs: 更新规范\n")
        result = self.check("--base", "HEAD", "--staged", "--message-file", str(self.repo / ".git/COMMIT_EDITMSG"))
        self.assertNotEqual(result.returncode, 0)
        self.write(".git/COMMIT_EDITMSG", "docs: 更新规范\n\n版本影响：无；仅调整规范说明，无产品行为变化\n")
        result = self.check("--base", "HEAD", "--staged", "--message-file", str(self.repo / ".git/COMMIT_EDITMSG"))
        self.assertEqual(result.returncode, 0)

    def test_node_local_framework_dependency_is_rejected(self):
        path = self.repo / "Cargo.toml"
        path.write_text(path.read_text().replace('miresidian = "=0.11.0"', 'miresidian = { version = "=0.11.0", path = "../framework" }'))
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("禁止 path/git", result.stderr)

    def test_node_local_override_and_path_lock_are_rejected(self):
        path = self.repo / "Cargo.toml"
        original = path.read_text()
        for override in ['\n[patch.crates-io]\nmiresidian = { path = "../framework" }\n', '\n[replace]\n"miresidian:0.11.0" = { package = "miresidian", path = "../framework" }\n']:
            path.write_text(original + override)
            self.assertNotEqual(self.check().returncode, 0)
        path.write_text(original)
        lock = self.repo / "Cargo.lock"
        lock.write_text(lock.read_text().replace('source = "registry+https://github.com/rust-lang/crates.io-index"\n', ''))
        self.assertNotEqual(self.check().returncode, 0)

    def test_current_and_legacy_instance_packages_keep_registry_source_checks(self):
        manifest = self.repo / "Cargo.toml"
        lock = self.repo / "Cargo.lock"
        original_manifest, original_lock = manifest.read_text(), lock.read_text()
        for name in ("miresidian_instance", "miresidian_instances"):
            with self.subTest(package=name):
                valid_manifest = original_manifest + f'{name} = "=0.17.0"\n'
                bare_lock = original_lock + f'\n[[package]]\nname = "{name}"\nversion = "0.17.0"\n'
                valid_lock = bare_lock + 'source = "registry+https://github.com/rust-lang/crates.io-index"\nchecksum = "' + 'a' * 64 + '"\n'
                manifest.write_text(valid_manifest)
                lock.write_text(valid_lock)
                self.assertEqual(self.check().returncode, 0)
                for source in ('path = "../local"', 'git = "https://example.invalid/core"'):
                    manifest.write_text(original_manifest + f'alias = {{ package = "{name}", version = "=0.17.0", {source} }}\n')
                    result = self.check()
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(name, result.stderr)
                for override in (f'\n[patch.crates-io]\nalias = {{ package = "{name}", path = "../local" }}\n',
                                 f'\n[replace]\n"{name}:0.17.0" = {{ path = "../local" }}\n'):
                    manifest.write_text(valid_manifest + override)
                    self.assertNotEqual(self.check().returncode, 0)
                manifest.write_text(valid_manifest)
                lock.write_text(bare_lock)
                result = self.check()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(name, result.stderr)

    def test_node_cargo_paths_override_is_rejected(self):
        self.write(".cargo/config.toml", 'paths = ["../framework"]\n')
        self.assertNotEqual(self.check().returncode, 0)

    def framework_release(self, version, abi_version, extra=False, dependency=None):
        self.write("Cargo.toml", f'''[package]
name = "miresidian"
version.workspace = true
[workspace]
members = ["abi"]
[workspace.package]
version = "{version}"
[dependencies]
miresidian_abi = {{ version = "={dependency or version}", path = "abi" }}
''')
        self.write("abi/Cargo.toml", f'''[package]
name = "miresidian_abi"
version.workspace = true
[package.metadata.miresidian]
abi_version = {abi_version}
''')
        self.write("Cargo.lock", f'''version = 4
[[package]]
name = "miresidian"
version = "{version}"
[[package]]
name = "miresidian_abi"
version = "{version}"
''')
        self.write("abi/src/lib.rs", f"pub const MIRESIDIAN_ABI_VERSION: u32 = {abi_version};\n")
        header = f"#define MIRESIDIAN_ABI_VERSION {abi_version}\nMIRESIDIAN_API uint32_t MIRESIDIAN_CALL miresidian_abi_version(void);\n"
        if extra:
            header += "MIRESIDIAN_API const char * MIRESIDIAN_CALL extra(void);\n"
        self.write("abi/api/miresidian.h", header)
        self.write("CHANGELOG.md", f"## [Unreleased]\n\n## [{version}] - 2026-10-29\n\n- 更新框架接口。\n")

    def test_pack_component_inventory_cannot_omit_or_duplicate_framework_libraries(self):
        self.framework_release("0.1.0", 7)
        for inventory in ["miresidian\n", "miresidian\nmiresidian_abi\nmiresidian_abi\n"]:
            self.write("tools/miresidian-pack/framework-crates.txt", inventory)
            result = self.check()
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("组件清单不一致", result.stderr)

    def test_c_export_change_requires_consistent_new_abi_version(self):
        self.framework_release("0.1.0", 7)
        self.commit()
        self.framework_release("0.2.0", 7, extra=True)
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("必须同步提升 ABI", result.stderr)
        self.framework_release("0.2.0", 8, extra=True)
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)
        self.write("abi/src/lib.rs", "pub const MIRESIDIAN_ABI_VERSION: u32 = 7;\n")
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("不一致", result.stderr)

    def test_tools_change_versions_independently_of_root(self):
        self.framework_release("0.1.0", 7)
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('members = ["abi"]', 'members = ["abi", "tool"]'))
        self.write("tool/Cargo.toml", '[package]\nname = "test_tool"\nversion = "0.1.0"\n')
        self.write("tool/src/main.rs", 'fn main() {}\n')
        lock = self.repo / "Cargo.lock"
        lock.write_text(lock.read_text() + '\n[[package]]\nname = "test_tool"\nversion = "0.1.0"\n')
        self.commit()
        self.write("tool/Cargo.toml", '[package]\nname = "test_tool"\nversion = "0.2.0"\n')
        self.write("tool/src/main.rs", 'fn main() { println!("updated"); }\n')
        lock.write_text(lock.read_text().replace('name = "test_tool"\nversion = "0.1.0"', 'name = "test_tool"\nversion = "0.2.0"'))
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)
        self.write("tool/Cargo.toml", '[package]\nname = "test_tool"\nversion = "0.1.0"\n')
        self.assertNotEqual(self.check("--base", "HEAD").returncode, 0)

    def test_component_tag_is_created_without_tagging_the_root(self):
        self.framework_release("0.1.0", 7)
        head = self.commit()
        self.assertEqual(self.check("--component", "miresidian_abi", "--tag").returncode, 0)
        self.assertEqual(self.git("rev-parse", "miresidian_abi-v0.1.0^{commit}"), head)
        self.assertEqual(self.check("--component", "miresidian_abi", "--require-tag").returncode, 0)
        self.assertNotIn("miresidian-v0.1.0", self.git("tag"))

    def test_history_accepts_component_tags_created_on_descendants(self):
        self.independent_framework("0.1.0", "0.1.0")
        base = self.commit()
        self.independent_framework("0.2.0", "0.2.0")
        self.commit()
        self.independent_framework("0.3.0", "0.2.0")
        self.commit()
        self.git("tag", "miresidian_abi-v0.2.0")
        # 原有 0.2.0 版本标签存在于以后提交，不应阻断当时没有该标签的历史检查。
        self.assertEqual(self.check("--base", base, "--check-commits").returncode, 0)

    def test_history_keeps_independent_component_release_tags(self):
        self.independent_framework("0.1.0", "0.1.0")
        base = self.commit()
        self.independent_framework("0.2.0", "0.2.0")
        self.commit()
        self.git("tag", "miresidian_abi-v0.2.0")
        self.independent_framework("0.3.0", "0.2.0")
        self.commit()
        self.assertEqual(self.check("--base", base, "--check-commits").returncode, 0)
        # 普通工作区检查仍拒绝复用一个指向其他提交的版本标签。
        self.assertNotEqual(self.check("--base", base).returncode, 0)

    def test_registry_policy_does_not_rewrite_old_local_dependency_history(self):
        base = self.git("rev-parse", "HEAD")
        self.release("0.2.0")
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('miresidian = "=0.11.0"', 'miresidian = { version = "=0.11.0", path = "../framework" }'))
        self.commit()
        self.release("0.3.0")
        self.write("AGENTS.md", "子节点禁止 path、git 和本地覆盖\n")
        self.commit()
        self.assertEqual(self.check("--base", base, "--check-commits").returncode, 0)

    def test_internal_dependency_must_match_target_crate_version(self):
        self.framework_release("0.1.0", 7)
        self.commit()
        self.framework_release("0.2.0", 7, dependency="0.1.0")
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("未精确依赖对应 crate 版本", result.stderr)

    def independent_framework(self, root_version, library_version):
        self.framework_release(root_version, 7, dependency=library_version)
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('version.workspace = true', f'version = "{root_version}"').replace(f'version = "{root_version}"\n[dependencies]', '[dependencies]'))
        library = self.repo / "abi/Cargo.toml"
        library.write_text(library.read_text().replace('version.workspace = true', f'version = "{library_version}"'))
        lock = self.repo / "Cargo.lock"
        lock.write_text(lock.read_text().replace(f'name = "miresidian_abi"\nversion = "{root_version}"', f'name = "miresidian_abi"\nversion = "{library_version}"'))
        self.write("abi/CHANGELOG.md", f"## [{library_version}] - 2026-10-30\n\n- 独立维护 ABI 库。\n")

    def test_root_update_does_not_bump_library_version(self):
        self.independent_framework("0.1.0", "0.1.0")
        self.commit()
        self.independent_framework("0.2.0", "0.1.0")
        self.write("src/lib.rs", "pub fn value() -> u32 { 2 }\n")
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)

    def test_library_update_does_not_bump_root_version(self):
        self.independent_framework("0.1.0", "0.1.0")
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('miresidian_abi = { version = "=0.1.0", path = "abi" }\n', ''))
        self.commit()
        library = self.repo / "abi/Cargo.toml"
        library.write_text(library.read_text().replace('version = "0.1.0"', 'version = "0.1.1"'))
        self.write("abi/src/lib.rs", "pub const MIRESIDIAN_ABI_VERSION: u32 = 7;\npub const NEW_API: bool = true;\n")
        self.write("abi/CHANGELOG.md", "## [0.1.1] - 2026-10-30\n\n- 修复独立库。\n")
        lock = self.repo / "Cargo.lock"
        lock.write_text(lock.read_text().replace('name = "miresidian_abi"\nversion = "0.1.0"', 'name = "miresidian_abi"\nversion = "0.1.1"'))
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)

    def test_existing_component_tag_cannot_be_reused(self):
        self.independent_framework("0.1.0", "0.1.0")
        self.commit()
        self.git("tag", "miresidian_abi-v0.2.0")
        self.independent_framework("0.2.0", "0.2.0")
        self.git("add", ".")
        result = self.check("--base", "HEAD", "--staged")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("禁止复用", result.stderr)

    def test_c_export_whitespace_does_not_require_abi_increment(self):
        self.framework_release("0.1.0", 7)
        self.commit()
        self.framework_release("0.2.0", 7)
        file = self.repo / "abi/api/miresidian.h"
        file.write_text(file.read_text().replace("(void)", "( void )"), encoding="utf-8")
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)

    def enable_guidance_policy(self):
        self.write("AGENTS.md", "# 指导性文档随组件版本更新\n")
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text() + '\n[package.metadata.docs.rs]\nall-features = true\n')

    def test_framework_release_requires_readme_and_contract_guide(self):
        self.independent_framework("0.1.0", "0.1.0")
        self.commit()
        self.independent_framework("0.1.1", "0.1.0")
        self.enable_guidance_policy()
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("README.md", result.stderr)
        self.write("README.md", "# 0.1.1\n")
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("核心技术手册", result.stderr)
        self.write("docs/宏驱动开发.md", "# 更新节点契约与失败清理\n")
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)

    def test_unstaged_guide_cannot_satisfy_staged_release(self):
        self.independent_framework("0.1.0", "0.1.0")
        self.commit()
        self.independent_framework("0.1.1", "0.1.0")
        self.enable_guidance_policy()
        self.write("README.md", "# 0.1.1\n")
        self.git("add", ".")
        self.write("docs/宏驱动开发.md", "# 尚未暂存的指导\n")
        result = self.check("--base", "HEAD", "--staged")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("核心技术手册", result.stderr)
        self.git("add", "docs/宏驱动开发.md")
        self.assertEqual(self.check("--base", "HEAD", "--staged").returncode, 0)

    def test_component_release_requires_its_own_readme(self):
        self.independent_framework("0.1.0", "0.1.0")
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('miresidian_abi = { version = "=0.1.0", path = "abi" }\n', ''))
        self.enable_guidance_policy()
        self.commit()
        self.independent_framework("0.1.0", "0.1.1")
        manifest.write_text(manifest.read_text().replace('miresidian_abi = { version = "=0.1.1", path = "abi" }\n', ''))
        self.enable_guidance_policy()
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("abi/README.md", result.stderr)
        self.write("docs/versioning.md", "# 维护规范不能代替组件说明\n")
        self.assertNotEqual(self.check("--base", "HEAD").returncode, 0)
        self.write("abi/README.md", "# 更新 ABI 库用法\n")
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)

    def test_docs_rs_must_cover_optional_public_api(self):
        self.independent_framework("0.1.0", "0.1.0")
        self.enable_guidance_policy()
        self.assertEqual(self.check().returncode, 0)
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('all-features = true', 'all-features = false'))
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("docs.rs", result.stderr)

    def test_pure_skill_update_remains_independent(self):
        self.independent_framework("0.1.0", "0.1.0")
        self.enable_guidance_policy()
        self.commit()
        self.write("skills/miresidian-node/SKILL.md", "# 更新通用开发流程\n")
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)

    def test_guidance_policy_keeps_older_release_history(self):
        self.independent_framework("0.1.0", "0.1.0")
        base = self.commit()
        self.independent_framework("0.1.1", "0.1.0")
        self.commit()  # 新规范生效前的提交没有新增 README/手册，不应被追溯阻断。
        self.independent_framework("0.1.2", "0.1.0")
        self.enable_guidance_policy()
        self.write("README.md", "# 当前发行\n")
        self.write("docs/宏驱动开发.md", "# 当前契约\n")
        self.commit()
        self.assertEqual(self.check("--base", base, "--check-commits").returncode, 0)

    def test_template_and_toolset_changes_require_tool_version(self):
        self.independent_framework("0.1.0", "0.1.0")
        manifest = self.repo / "Cargo.toml"
        manifest.write_text(manifest.read_text().replace('members = ["abi"]', 'members = ["abi", "tools/helper"]'))
        self.write("tools/helper/Cargo.toml", '[package]\nname = "helper"\nversion = "0.1.0"\n')
        self.write("tools/helper/src/main.rs", 'fn main() {}\n')
        lock = self.repo / "Cargo.lock"
        lock.write_text(lock.read_text() + '\n[[package]]\nname = "helper"\nversion = "0.1.0"\n')
        self.enable_guidance_policy()
        self.commit()
        for name in ('templates/AGENTS.md.in', 'toolset.toml'):
            with self.subTest(path=name):
                self.write('tools/helper/' + name, 'changed guidance\n')
                result = self.check("--base", "HEAD")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("必须独立提升", result.stderr)
                (self.repo / 'tools/helper' / name).unlink()


if __name__ == "__main__":
    unittest.main()
