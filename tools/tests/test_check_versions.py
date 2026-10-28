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

    def test_internal_dependency_must_follow_release_train(self):
        self.framework_release("0.1.0", 7)
        self.commit()
        self.framework_release("0.2.0", 7, dependency="0.1.0")
        result = self.check("--base", "HEAD")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("未精确依赖发布列车", result.stderr)

    def test_c_export_whitespace_does_not_require_abi_increment(self):
        self.framework_release("0.1.0", 7)
        self.commit()
        self.framework_release("0.2.0", 7)
        file = self.repo / "abi/api/miresidian.h"
        file.write_text(file.read_text().replace("(void)", "( void )"), encoding="utf-8")
        self.assertEqual(self.check("--base", "HEAD").returncode, 0)


if __name__ == "__main__":
    unittest.main()
