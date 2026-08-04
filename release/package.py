#!/usr/bin/env python3

import argparse
import hashlib
import re
import shutil
import sys
import tarfile
import tempfile
import tomllib
import zipfile
from pathlib import Path, PurePosixPath


TAG_PATTERN = re.compile(
    r"complexity-v(?P<version>(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*))"
)
TARGET_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]*")
AGENT_MANIFEST = "MANIFEST.txt"


class PackageError(Exception):
    pass


def cargo_version(project_root: Path) -> str:
    manifest_path = project_root / "Cargo.toml"
    try:
        with manifest_path.open("rb") as manifest:
            data = tomllib.load(manifest)
        version = data["package"]["version"]
    except (OSError, KeyError, tomllib.TOMLDecodeError) as error:
        raise PackageError(f"cannot read package version from {manifest_path}: {error}") from error

    if not isinstance(version, str):
        raise PackageError(f"package version in {manifest_path} must be a string")
    return version


def validate_tag(tag: str, project_root: Path) -> str:
    match = TAG_PATTERN.fullmatch(tag)
    if match is None:
        raise PackageError("tag must use the form complexity-vX.Y.Z")

    tag_version = match.group("version")
    manifest_version = cargo_version(project_root)
    if tag_version != manifest_version:
        raise PackageError(
            f"tag version {tag_version} does not match Cargo.toml version {manifest_version}"
        )
    return tag_version


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as contents:
        for chunk in iter(lambda: contents.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require_file(path: Path, label: str) -> None:
    if not path.is_file():
        raise PackageError(f"{label} does not exist or is not a file: {path}")


def validate_package_inputs(
    binary: Path, readme: Path, license_file: Path, agent_directory: Path
) -> None:
    require_file(binary, "binary")
    require_file(readme, "README")
    require_file(license_file, "LICENSE")
    if not agent_directory.is_dir() or agent_directory.is_symlink():
        raise PackageError(f"agent tree does not exist or is not a directory: {agent_directory}")


def archive_details(target: str) -> tuple[str, str, bool]:
    if "-windows-" in target:
        return ".zip", "complexity.exe", True
    return ".tar.gz", "complexity", False


def agent_manifest_entries(agent_directory: Path) -> list[str]:
    manifest = agent_directory / AGENT_MANIFEST
    if manifest.is_symlink():
        raise PackageError(f"agent manifest must not be a symlink: {manifest}")
    try:
        entries = manifest.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise PackageError(f"cannot read agent manifest {manifest}: {error}") from error
    if entries != sorted(set(entries)) or AGENT_MANIFEST not in entries:
        raise PackageError("agent manifest must be a sorted unique list that includes itself")
    return entries


def agent_source(agent_directory: Path, entry: str) -> tuple[Path, PurePosixPath]:
    relative = PurePosixPath(entry)
    if relative.is_absolute() or not entry or any(part in {".", ".."} for part in relative.parts):
        raise PackageError(f"unsafe agent manifest path: {entry}")
    source = agent_directory
    for part in relative.parts:
        source /= part
        if source.is_symlink():
            raise PackageError(f"agent manifest path uses a symlink: {entry}")
    try:
        source.resolve(strict=True).relative_to(agent_directory.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise PackageError(f"agent manifest path leaves the agent tree: {entry}") from error
    require_file(source, "agent manifest entry")
    return source, relative


def copy_agent_tree(agent_directory: Path, destination: Path) -> None:
    for entry in agent_manifest_entries(agent_directory):
        source, relative = agent_source(agent_directory, entry)
        target = destination.joinpath(*relative.parts)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)


def populate_archive_root(
    archive_root: Path,
    binary: Path,
    binary_name: str,
    readme: Path,
    license_file: Path,
    agent_directory: Path,
) -> None:
    archive_root.mkdir()
    shutil.copy2(binary, archive_root / binary_name)
    shutil.copy2(readme, archive_root / "README.md")
    shutil.copy2(license_file, archive_root / "LICENSE")
    copy_agent_tree(agent_directory, archive_root / "agent")


def write_archive(archive: Path, archive_root: Path, archive_base: str, is_windows: bool) -> None:
    if is_windows:
        with zipfile.ZipFile(
            archive, "w", compression=zipfile.ZIP_DEFLATED
        ) as release_archive:
            for path in sorted(archive_root.rglob("*")):
                archive_path = path.relative_to(archive_root.parent).as_posix()
                release_archive.write(path, archive_path)
        return
    with tarfile.open(archive, "w:gz") as release_archive:
        release_archive.add(archive_root, arcname=archive_base)


def write_checksum(archive: Path) -> Path:
    checksum = archive.with_name(f"{archive.name}.sha256")
    with checksum.open("w", encoding="utf-8", newline="\n") as checksum_file:
        checksum_file.write(f"{sha256_file(archive)}  {archive.name}\n")
    return checksum


def package_release(
    tag: str,
    target: str,
    binary: Path,
    project_root: Path,
    output_directory: Path,
) -> tuple[Path, Path]:
    version = validate_tag(tag, project_root)
    if TARGET_PATTERN.fullmatch(target) is None:
        raise PackageError(f"invalid target triple: {target}")

    readme = project_root / "README.md"
    license_file = project_root / "LICENSE"
    agent_directory = project_root / "agent"
    validate_package_inputs(binary, readme, license_file, agent_directory)

    archive_base = f"complexity-{version}-{target}"
    archive_suffix, binary_name, is_windows = archive_details(target)
    output_directory.mkdir(parents=True, exist_ok=True)
    archive = output_directory / f"{archive_base}{archive_suffix}"

    with tempfile.TemporaryDirectory() as temporary_directory:
        archive_root = Path(temporary_directory) / archive_base
        populate_archive_root(
            archive_root, binary, binary_name, readme, license_file, agent_directory
        )
        write_archive(archive, archive_root, archive_base, is_windows)

    return archive, write_checksum(archive)


def parse_arguments(arguments: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build complexity release archives")
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser("validate-tag")
    validate_parser.add_argument("--tag", required=True)
    validate_parser.add_argument(
        "--project-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
    )

    package_parser = subparsers.add_parser("package")
    package_parser.add_argument("--tag", required=True)
    package_parser.add_argument("--target", required=True)
    package_parser.add_argument("--binary", required=True, type=Path)
    package_parser.add_argument(
        "--project-root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
    )
    package_parser.add_argument("--output-dir", required=True, type=Path)
    return parser.parse_args(arguments)


def run(arguments: list[str]) -> int:
    options = parse_arguments(arguments)
    if options.command == "validate-tag":
        print(validate_tag(options.tag, options.project_root))
        return 0
    if options.command == "package":
        archive, checksum = package_release(
            options.tag,
            options.target,
            options.binary,
            options.project_root,
            options.output_dir,
        )
        print(archive)
        print(checksum)
        return 0
    raise AssertionError(f"unsupported command: {options.command}")


def main() -> int:
    try:
        return run(sys.argv[1:])
    except PackageError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
