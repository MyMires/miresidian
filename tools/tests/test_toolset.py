"""完整套件安装的可观察行为：校验先于写入、宿主匹配及路径边界。"""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1] / "miresidian-cli/distribution/install.py"
SPEC = importlib.util.spec_from_file_location("miresidian_installer", SOURCE)
INSTALLER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALLER)


class ToolsetTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="miresidian-suite-")
        self.root = Path(self.temporary.name)
        self.archive = self.root / "tools.tar.gz"
        self.destination = self.root / "install with spaces"
        self.files = {name: b"independent executable" for name in INSTALLER.REQUIRED}

    def tearDown(self):
        self.temporary.cleanup()

    def package(self, extra=None, wrong_file_hash=False):
        tools = [{"package": "fixture", "version": "0.1.0", "files": [
            {"name": name, "sha256": "0" * 64 if wrong_file_hash else hashlib.sha256(data).hexdigest()}
            for name, data in self.files.items()
        ]}]
        manifest = {"format_version": 1, "version": "0.1.0", "host": "linux-x64", "tools": tools}
        files = dict(self.files, **{"manifest.json": json.dumps(manifest).encode()})
        if extra:
            files.update(extra)
        with tarfile.open(self.archive, "w:gz") as package:
            for name, data in files.items():
                member = tarfile.TarInfo(name)
                member.size = len(data)
                package.addfile(member, io.BytesIO(data))
        self.archive.with_name(self.archive.name + ".sha256").write_text(INSTALLER.digest(self.archive) + "  tools.tar.gz\n")

    def test_complete_install_and_independent_binaries(self):
        self.package()
        INSTALLER.install(self.archive, self.destination, "linux-x64", "0.1.0")
        for name, data in self.files.items():
            self.assertEqual((self.destination / name).read_bytes(), data)
            self.assertTrue((self.destination / name).stat().st_mode & 0o111)

    def test_wrong_archive_or_file_digest_does_not_change_existing_install(self):
        self.destination.mkdir()
        existing = self.destination / "cargo-mires"
        existing.write_bytes(b"existing")
        self.package(wrong_file_hash=True)
        with self.assertRaises(ValueError):
            INSTALLER.install(self.archive, self.destination, "linux-x64")
        self.assertEqual(existing.read_bytes(), b"existing")
        self.assertEqual(len(list(self.destination.iterdir())), 1)
        self.package()
        self.archive.write_bytes(self.archive.read_bytes() + b"tampered")
        with self.assertRaises(ValueError):
            INSTALLER.install(self.archive, self.destination, "linux-x64")
        self.assertEqual(existing.read_bytes(), b"existing")

    def test_path_traversal_and_host_mismatch_are_rejected(self):
        self.package({"../escape": b"bad"})
        with self.assertRaises(ValueError):
            INSTALLER.install(self.archive, self.destination, "linux-x64")
        self.assertFalse((self.root / "escape").exists())
        self.assertFalse(self.destination.exists())
        self.package()
        with self.assertRaises(ValueError):
            INSTALLER.install(self.archive, self.destination, "windows-x64")
        self.assertFalse(self.destination.exists())

    def test_latest_selects_toolset_semver_only(self):
        releases = [
            {"tag_name": tag, "draft": draft, "prerelease": prerelease}
            for tag, draft, prerelease in [
                ("miresidian-v9.0.0", False, False),
                ("miresidian_cli-v0.2.0", False, False),
                ("miresidian_cli-v0.10.0", False, False),
                ("miresidian_cli-v1.0.0", True, False),
                ("miresidian_cli-v2.0.0", False, True),
                ("miresidian_cli-v3.0.0-beta.1", False, True),
            ]
        ]
        with patch.object(INSTALLER.urllib.request, "urlopen", return_value=io.BytesIO(json.dumps(releases).encode())):
            self.assertEqual(INSTALLER.latest(), "0.10.0")

    def test_zip_install_and_links_are_rejected(self):
        self.package()
        archive = self.root / "tools.zip"
        with tarfile.open(self.archive) as source, zipfile.ZipFile(archive, "w") as target:
            for member in source.getmembers():
                with source.extractfile(member) as stream:
                    target.writestr(member.name, stream.read())
        checksum = archive.with_name(archive.name + ".sha256")
        checksum.write_text(INSTALLER.digest(archive))
        INSTALLER.install(archive, self.destination, "linux-x64")
        original = (self.destination / "cargo-mires").read_bytes()
        with zipfile.ZipFile(archive, "a") as target:
            link = zipfile.ZipInfo("cargo-evil")
            link.create_system = 3
            link.external_attr = 0o120777 << 16
            target.writestr(link, "../escape")
        checksum.write_text(INSTALLER.digest(archive))
        with self.assertRaises(ValueError):
            INSTALLER.install(archive, self.destination, "linux-x64")
        self.assertEqual((self.destination / "cargo-mires").read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
