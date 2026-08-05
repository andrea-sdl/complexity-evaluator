#!/usr/bin/env python3

import argparse
import gzip
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
PLUGIN_PATH = PurePosixPath("plugins/complexity-evaluator")
MARKETPLACE_PATHS = (
    PurePosixPath(".agents/plugins/marketplace.json"),
    PurePosixPath(".claude-plugin/marketplace.json"),
)
README_IMAGES = (
    PurePosixPath("docs/images/complexity-hero.jpg"),
    PurePosixPath("docs/images/find-risky-functions.jpg"),
    PurePosixPath("docs/images/one-policy-many-languages.jpg"),
    PurePosixPath("docs/images/refactor-with-proof.jpg"),
)


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


def path_uses_symlink(project_root: Path, path: Path) -> bool:
    current = project_root
    for part in path.relative_to(project_root).parts:
        current /= part
        if current.is_symlink():
            return True
    return False


def require_tree(path: Path, label: str, project_root: Path) -> None:
    if not path.is_dir() or path_uses_symlink(project_root, path):
        raise PackageError(f"{label} does not exist or is not a directory: {path}")


def require_named_files(
    files: list[tuple[Path, PurePosixPath]], label: str, project_root: Path
) -> None:
    for path, relative in files:
        if path_uses_symlink(project_root, path):
            raise PackageError(f"{label} path uses a symlink: {relative}")
        require_file(path, label)


def validate_package_inputs(
    binary: Path,
    readme: Path,
    license_file: Path,
    agent_directory: Path,
    plugin_directory: Path,
    marketplaces: list[tuple[Path, PurePosixPath]],
    readme_images: list[tuple[Path, PurePosixPath]],
) -> None:
    require_file(binary, "binary")
    require_file(readme, "README")
    require_file(license_file, "LICENSE")
    project_root = readme.parent
    require_tree(agent_directory, "agent tree", project_root)
    require_tree(plugin_directory, "plugin tree", project_root)
    require_named_files(marketplaces, "marketplace", project_root)
    require_named_files(readme_images, "README image", project_root)


def archive_details(target: str) -> tuple[str, str, bool]:
    if "-windows-" in target:
        return ".zip", "complexity.exe", True
    return ".tar.gz", "complexity", False


def tree_manifest_entries(tree_directory: Path, label: str) -> list[str]:
    manifest = tree_directory / AGENT_MANIFEST
    if manifest.is_symlink():
        raise PackageError(f"{label} manifest must not be a symlink: {manifest}")
    try:
        entries = manifest.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise PackageError(f"cannot read {label} manifest {manifest}: {error}") from error
    if entries != sorted(set(entries)) or AGENT_MANIFEST not in entries:
        raise PackageError(f"{label} manifest must be a sorted unique list that includes itself")
    return entries


def tree_source(
    tree_directory: Path, entry: str, label: str
) -> tuple[Path, PurePosixPath]:
    relative = PurePosixPath(entry)
    if relative.is_absolute() or not entry or any(part in {".", ".."} for part in relative.parts):
        raise PackageError(f"unsafe {label} manifest path: {entry}")
    source = tree_directory
    for part in relative.parts:
        source /= part
        if source.is_symlink():
            raise PackageError(f"{label} manifest path uses a symlink: {entry}")
    try:
        source.resolve(strict=True).relative_to(tree_directory.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise PackageError(f"{label} manifest path leaves the tree: {entry}") from error
    require_file(source, f"{label} manifest entry")
    return source, relative


def copy_manifest_tree(tree_directory: Path, destination: Path, label: str) -> None:
    for entry in tree_manifest_entries(tree_directory, label):
        source, relative = tree_source(tree_directory, entry, label)
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
    plugin_directory: Path,
    marketplaces: list[tuple[Path, PurePosixPath]],
    readme_images: list[tuple[Path, PurePosixPath]],
) -> None:
    archive_root.mkdir()
    shutil.copy2(binary, archive_root / binary_name)
    shutil.copy2(readme, archive_root / "README.md")
    shutil.copy2(license_file, archive_root / "LICENSE")
    for image, relative in readme_images:
        target = archive_root.joinpath(*relative.parts)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(image, target)
    copy_manifest_tree(agent_directory, archive_root / "agent", "agent")
    copy_manifest_tree(
        plugin_directory,
        archive_root.joinpath(*PLUGIN_PATH.parts),
        "plugin",
    )
    for marketplace, relative in marketplaces:
        target = archive_root.joinpath(*relative.parts)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(marketplace, target)


def archive_paths(archive_root: Path) -> list[Path]:
    return [archive_root, *sorted(archive_root.rglob("*"))]


def normalized_tar_info(info: tarfile.TarInfo, archive_base: str) -> tarfile.TarInfo:
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    info.mtime = 0
    info.pax_headers = {}
    binary_names = {f"{archive_base}/complexity", f"{archive_base}/complexity.exe"}
    info.mode = 0o755 if info.isdir() or info.name in binary_names else 0o644
    return info


def write_tar_archive(archive: Path, archive_root: Path, archive_base: str) -> None:
    with archive.open("wb") as archive_file:
        with gzip.GzipFile(
            filename="", mode="wb", fileobj=archive_file, compresslevel=9, mtime=0
        ) as compressed:
            with tarfile.open(
                fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT
            ) as release_archive:
                for path in archive_paths(archive_root):
                    archive_path = path.relative_to(archive_root.parent).as_posix()
                    release_archive.add(
                        path,
                        arcname=archive_path,
                        recursive=False,
                        filter=lambda info: normalized_tar_info(info, archive_base),
                    )


def zip_info(path: Path, archive_root: Path, archive_base: str) -> zipfile.ZipInfo:
    archive_path = path.relative_to(archive_root.parent).as_posix()
    if path.is_dir():
        archive_path += "/"
    info = zipfile.ZipInfo(archive_path, date_time=(1980, 1, 1, 0, 0, 0))
    info.create_system = 3
    info.compress_type = zipfile.ZIP_STORED if path.is_dir() else zipfile.ZIP_DEFLATED
    binary_names = {f"{archive_base}/complexity", f"{archive_base}/complexity.exe"}
    mode = 0o755 if path.is_dir() or archive_path in binary_names else 0o644
    file_type = 0o040000 if path.is_dir() else 0o100000
    info.external_attr = (file_type | mode) << 16
    return info


def write_zip_archive(archive: Path, archive_root: Path, archive_base: str) -> None:
    with zipfile.ZipFile(archive, "w") as release_archive:
        for path in archive_paths(archive_root):
            contents = b"" if path.is_dir() else path.read_bytes()
            release_archive.writestr(zip_info(path, archive_root, archive_base), contents)


def write_archive(archive: Path, archive_root: Path, archive_base: str, is_windows: bool) -> None:
    if is_windows:
        write_zip_archive(archive, archive_root, archive_base)
        return
    write_tar_archive(archive, archive_root, archive_base)


def write_checksum(archive: Path) -> Path:
    checksum = archive.with_name(f"{archive.name}.sha256")
    with checksum.open("w", encoding="utf-8", newline="\n") as checksum_file:
        checksum_file.write(f"{sha256_file(archive)}  {archive.name}\n")
    return checksum


def marketplace_files(project_root: Path) -> list[tuple[Path, PurePosixPath]]:
    return [
        (project_root.joinpath(*relative.parts), relative)
        for relative in MARKETPLACE_PATHS
    ]


def readme_image_files(project_root: Path) -> list[tuple[Path, PurePosixPath]]:
    return [
        (project_root.joinpath(*relative.parts), relative)
        for relative in README_IMAGES
    ]


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
    plugin_directory = project_root.joinpath(*PLUGIN_PATH.parts)
    marketplaces = marketplace_files(project_root)
    readme_images = readme_image_files(project_root)
    validate_package_inputs(
        binary,
        readme,
        license_file,
        agent_directory,
        plugin_directory,
        marketplaces,
        readme_images,
    )

    archive_base = f"complexity-{version}-{target}"
    archive_suffix, binary_name, is_windows = archive_details(target)
    output_directory.mkdir(parents=True, exist_ok=True)
    archive = output_directory / f"{archive_base}{archive_suffix}"

    with tempfile.TemporaryDirectory() as temporary_directory:
        archive_root = Path(temporary_directory) / archive_base
        populate_archive_root(
            archive_root,
            binary,
            binary_name,
            readme,
            license_file,
            agent_directory,
            plugin_directory,
            marketplaces,
            readme_images,
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
