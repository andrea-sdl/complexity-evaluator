import hashlib
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path


SCRIPT = Path(__file__).parents[1] / "package.py"


def write_text_file(project_root: Path, relative_path: str, contents: str) -> Path:
    path = project_root / relative_path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents, encoding="utf-8")
    return path


def write_agent_manifest(project_root: Path, paths: list[str]) -> None:
    entries = sorted(["MANIFEST.txt", *paths])
    write_text_file(project_root, "agent/MANIFEST.txt", "\n".join(entries) + "\n")


def package_fixture(
    project_root: Path,
    version: str,
    binary_name: str,
    binary_contents: bytes,
    readme_contents: str,
) -> tuple[Path, Path]:
    write_text_file(
        project_root,
        "Cargo.toml",
        f'[package]\nname = "complexity"\nversion = "{version}"\n',
    )
    write_text_file(project_root, "README.md", readme_contents)
    write_text_file(project_root, "LICENSE", "license terms\n")
    binary = project_root / "target" / "release" / binary_name
    binary.parent.mkdir(parents=True)
    binary.write_bytes(binary_contents)
    return binary, project_root / "dist"


def run_package(
    project_root: Path,
    version: str,
    target: str,
    binary: Path,
    output_directory: Path,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            SCRIPT,
            "package",
            "--tag",
            f"complexity-v{version}",
            "--target",
            target,
            "--binary",
            binary,
            "--project-root",
            project_root,
            "--output-dir",
            output_directory,
        ],
        capture_output=True,
        check=False,
        text=True,
    )


class PackageCliTests(unittest.TestCase):
    def assert_archive_and_checksum(
        self, result: subprocess.CompletedProcess[str], archive: Path
    ) -> None:
        checksum = archive.with_name(f"{archive.name}.sha256")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(archive.is_file())
        self.assertTrue(checksum.is_file())
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        self.assertEqual(checksum.read_text(encoding="utf-8"), f"{digest}  {archive.name}\n")

    def assert_tar_contents(
        self, archive: Path, archive_root: str, expected_contents: dict[str, bytes]
    ) -> None:
        with tarfile.open(archive, "r:gz") as release_archive:
            for relative_path, contents in expected_contents.items():
                archived_file = release_archive.extractfile(f"{archive_root}/{relative_path}")
                self.assertIsNotNone(archived_file)
                assert archived_file is not None
                self.assertEqual(archived_file.read(), contents)

    def assert_zip_contents(
        self, archive: Path, archive_root: str, expected_contents: dict[str, bytes]
    ) -> None:
        with zipfile.ZipFile(archive) as release_archive:
            for relative_path, contents in expected_contents.items():
                self.assertEqual(release_archive.read(f"{archive_root}/{relative_path}"), contents)

    def test_validate_tag_accepts_the_cargo_package_version(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            (project_root / "Cargo.toml").write_text(
                '[package]\nname = "complexity"\nversion = "1.2.3"\n',
                encoding="utf-8",
            )

            result = subprocess.run(
                [
                    sys.executable,
                    SCRIPT,
                    "validate-tag",
                    "--tag",
                    "complexity-v1.2.3",
                    "--project-root",
                    project_root,
                ],
                capture_output=True,
                check=False,
                text=True,
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "1.2.3\n")

    def test_validate_tag_rejects_a_version_that_does_not_match_cargo(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            (project_root / "Cargo.toml").write_text(
                '[package]\nname = "complexity"\nversion = "1.2.3"\n',
                encoding="utf-8",
            )

            result = subprocess.run(
                [
                    sys.executable,
                    SCRIPT,
                    "validate-tag",
                    "--tag",
                    "complexity-v1.2.4",
                    "--project-root",
                    project_root,
                ],
                capture_output=True,
                check=False,
                text=True,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "tag version 1.2.4 does not match Cargo.toml version 1.2.3",
            result.stderr,
        )

    def test_package_rejects_an_agent_manifest_path_outside_the_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            binary, output_directory = package_fixture(
                project_root,
                version="1.2.3",
                binary_name="complexity",
                binary_contents=b"native binary",
                readme_contents="release readme\n",
            )
            write_text_file(
                project_root,
                "agent/MANIFEST.txt",
                "../README.md\nMANIFEST.txt\n",
            )

            result = run_package(
                project_root,
                "1.2.3",
                "x86_64-unknown-linux-gnu",
                binary,
                output_directory,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unsafe agent manifest path: ../README.md", result.stderr)

    def test_package_rejects_a_symlinked_agent_manifest_parent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            binary, output_directory = package_fixture(
                project_root,
                version="1.2.3",
                binary_name="complexity",
                binary_contents=b"native binary",
                readme_contents="release readme\n",
            )
            write_text_file(
                project_root,
                "agent/eval/scripts/eval-lib.mjs",
                "// eval support\n",
            )
            (project_root / "agent" / "alias").symlink_to(
                "eval/scripts",
                target_is_directory=True,
            )
            write_agent_manifest(project_root, ["alias/eval-lib.mjs"])

            result = run_package(
                project_root,
                "1.2.3",
                "x86_64-unknown-linux-gnu",
                binary,
                output_directory,
            )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "agent manifest path uses a symlink: alias/eval-lib.mjs",
            result.stderr,
        )

    def test_package_creates_a_unix_archive_with_the_agent_tree_and_checksum(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            binary, output_directory = package_fixture(
                project_root,
                version="1.2.3",
                binary_name="complexity",
                binary_contents=b"native binary",
                readme_contents="release readme\n",
            )
            write_text_file(
                project_root,
                "agent/skills/complexity-cli/SKILL.md",
                "skill contents\n",
            )
            write_text_file(project_root, "agent/hooks/codex.json", "{}\n")
            write_agent_manifest(
                project_root,
                ["hooks/codex.json", "skills/complexity-cli/SKILL.md"],
            )
            write_text_file(
                project_root,
                "agent/skills/complexity-cli/__pycache__/generated.pyc",
                "generated cache\n",
            )

            target = "x86_64-unknown-linux-gnu"
            result = run_package(project_root, "1.2.3", target, binary, output_directory)
            archive = output_directory / "complexity-1.2.3-x86_64-unknown-linux-gnu.tar.gz"
            archive_root = "complexity-1.2.3-x86_64-unknown-linux-gnu"
            self.assert_archive_and_checksum(result, archive)
            self.assert_tar_contents(
                archive,
                archive_root,
                {
                    "complexity": b"native binary",
                    "README.md": b"release readme\n",
                    "LICENSE": b"license terms\n",
                    "agent/skills/complexity-cli/SKILL.md": b"skill contents\n",
                    "agent/hooks/codex.json": b"{}\n",
                },
            )
            with tarfile.open(archive, "r:gz") as release_archive:
                self.assertNotIn(
                    f"{archive_root}/agent/skills/complexity-cli/__pycache__/generated.pyc",
                    release_archive.getnames(),
                )

    def test_package_creates_a_windows_zip_with_an_exe_binary(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            binary, output_directory = package_fixture(
                project_root,
                version="2.0.1",
                binary_name="complexity.exe",
                binary_contents=b"windows binary",
                readme_contents="readme\n",
            )
            write_text_file(project_root, "agent/eval/promptfooconfig.yaml", "tests: []\n")
            write_agent_manifest(project_root, ["eval/promptfooconfig.yaml"])

            target = "x86_64-pc-windows-msvc"
            result = run_package(project_root, "2.0.1", target, binary, output_directory)
            archive = output_directory / "complexity-2.0.1-x86_64-pc-windows-msvc.zip"
            archive_root = "complexity-2.0.1-x86_64-pc-windows-msvc"
            self.assert_archive_and_checksum(result, archive)
            self.assert_zip_contents(
                archive,
                archive_root,
                {
                    "complexity.exe": b"windows binary",
                    "README.md": b"readme\n",
                    "LICENSE": b"license terms\n",
                    "agent/eval/promptfooconfig.yaml": b"tests: []\n",
                },
            )


if __name__ == "__main__":
    unittest.main()
