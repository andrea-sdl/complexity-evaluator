#!/usr/bin/env python3
"""Generate the installable Complexity Evaluator plugin payload."""

import argparse
import json
import os
import shutil
import sys
import tomllib
from pathlib import Path, PurePosixPath


PROJECT_ROOT = Path(__file__).resolve().parents[1]
AGENT_ROOT = PROJECT_ROOT / "agent"
PLUGIN_NAME = "complexity-evaluator"
PLUGIN_ROOT = PROJECT_ROOT / "plugins" / PLUGIN_NAME
REPOSITORY = "https://github.com/andrea-sdl/complexity-evaluator"
AUTHOR = {"name": "Andrea Grassi", "url": "https://github.com/andrea-sdl"}


class SyncError(Exception):
    pass


def package_version() -> str:
    with (PROJECT_ROOT / "Cargo.toml").open("rb") as cargo_file:
        package = tomllib.load(cargo_file)["package"]
    version = package["version"]
    if not isinstance(version, str):
        raise SyncError("Cargo.toml package version must be a string")
    return version


def manifest_entries() -> list[PurePosixPath]:
    manifest = AGENT_ROOT / "MANIFEST.txt"
    entries = manifest.read_text(encoding="utf-8").splitlines()
    selected = [
        safe_agent_manifest_path(entry)
        for entry in entries
        if entry.startswith(("skills/", "eval/"))
    ]
    if entries != sorted(set(entries)) or not selected:
        raise SyncError("agent manifest must be sorted, unique, and include skill files")
    return selected


def safe_agent_manifest_path(entry: str) -> PurePosixPath:
    relative = PurePosixPath(entry)
    unsafe_part = any(part in {".", ".."} or ":" in part for part in relative.parts)
    portable = relative.as_posix() == entry and "\\" not in entry
    if relative.is_absolute() or unsafe_part or not portable:
        raise SyncError(f"unsafe agent manifest path: {entry}")
    return relative


def agent_source(entry: PurePosixPath) -> Path:
    source = AGENT_ROOT
    if source.is_symlink():
        raise SyncError("agent source tree uses a symlink")
    for part in entry.parts:
        source /= part
        if source.is_symlink():
            raise SyncError(f"agent source path uses a symlink: {entry}")
    try:
        source.resolve(strict=True).relative_to(AGENT_ROOT.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise SyncError(f"agent source path leaves the tree: {entry}") from error
    if not source.is_file():
        raise SyncError(f"agent source is not a regular file: {entry}")
    return source


def source_bytes(entries: list[PurePosixPath]) -> dict[PurePosixPath, bytes]:
    payload: dict[PurePosixPath, bytes] = {}
    for entry in entries:
        payload[entry] = agent_source(entry).read_bytes()
    return payload


def hook_payload(host: str, windows: bool) -> bytes:
    suffix = "-windows" if windows else ""
    sample = AGENT_ROOT / "hooks" / f"{host}{suffix}.json"
    data = json.loads(sample.read_text(encoding="utf-8"))
    data.pop("skillOverrides", None)
    launcher = "py -3" if windows else "python3"
    root = "%CLAUDE_PLUGIN_ROOT%" if windows else "${CLAUDE_PLUGIN_ROOT}"
    separator = "\\" if windows else "/"
    checker = f"{root}{separator}skills{separator}complexity-cli{separator}scripts{separator}check_complexity.py"
    for event in ("UserPromptSubmit", "Stop"):
        command = data["hooks"][event][0]["hooks"][0]
        command["command"] = f'{launcher} "{checker}" {"--baseline-hook" if event == "UserPromptSubmit" else "--hook"}'
    return json_bytes(data)


def json_bytes(data: object) -> bytes:
    return (json.dumps(data, indent=2, sort_keys=True) + "\n").encode("utf-8")


def plugin_manifest(version: str, host: str) -> dict[str, object]:
    manifest: dict[str, object] = {
        "name": PLUGIN_NAME,
        "version": version,
        "description": "Explicit skill for checking code complexity before handoff.",
        "author": AUTHOR,
        "homepage": REPOSITORY,
        "repository": REPOSITORY,
        "license": "GPL-2.0-only",
        "keywords": ["complexity", "cognitive-complexity", "code-quality"],
    }
    if host == "codex":
        manifest["skills"] = "./skills/"
        manifest["interface"] = {
            "displayName": "Complexity Evaluator",
            "shortDescription": "Check code complexity before handoff",
            "longDescription": "Measure changed code, name risky functions, and verify focused refactors.",
            "developerName": "Andrea Grassi",
            "category": "Productivity",
            "capabilities": ["Interactive"],
            "defaultPrompt": ["Use $complexity-cli to check changed supported code."],
        }
    return manifest


def marketplace_payload() -> dict[PurePosixPath, bytes]:
    codex = {
        "name": PLUGIN_NAME,
        "interface": {"displayName": "Complexity Evaluator"},
        "plugins": [{
            "name": PLUGIN_NAME,
            "source": {"source": "local", "path": "./plugins/complexity-evaluator"},
            "category": "Productivity",
            "policy": {"installation": "AVAILABLE", "authentication": "ON_INSTALL"},
        }],
    }
    claude = {
        "name": PLUGIN_NAME,
        "owner": {"name": "Andrea Grassi"},
        "description": "Explicit skill for checking code complexity before handoff.",
        "plugins": [{
            "name": PLUGIN_NAME,
            "source": "./plugins/complexity-evaluator",
            "description": "Check code complexity before handoff.",
            "category": "development",
            "tags": ["complexity", "code-quality"],
        }],
    }
    return {
        PurePosixPath(".agents/plugins/marketplace.json"): json_bytes(codex),
        PurePosixPath(".claude-plugin/marketplace.json"): json_bytes(claude),
    }


def plugin_readme() -> bytes:
    text = """# Complexity Evaluator plugin

This plugin exposes `$complexity-cli` only when you invoke it. Install the
`complexity` binary separately. A missing binary reports `BLOCKED`.

## Install for Codex

```sh
codex plugin marketplace add andrea-sdl/complexity-evaluator --ref main
codex plugin add complexity-evaluator@complexity-evaluator
```

Start a new Codex session, then invoke `$complexity-cli`.

## Install for Claude Code

```sh
claude plugin marketplace add andrea-sdl/complexity-evaluator
claude plugin install complexity-evaluator@complexity-evaluator
```

Run `/reload-plugins` or start a new session, then invoke
`/complexity-evaluator:complexity-cli`.

## Optional hooks

The base plugin does not enable hooks. In a source checkout or unpacked plugin,
copy one matching sample before you add the local marketplace or reload the
plugin:

```sh
cp hooks/codex.json hooks/hooks.json
```

Choose only one of `codex.json`, `codex-windows.json`, `claude.json`, or
`claude-windows.json`. On Windows, use `copy` instead of `cp`. The samples use
the host-provided `CLAUDE_PLUGIN_ROOT` variable and keep the checked repository
as the working directory.

Do not hand-edit generated files. Edit `agent/` in the source repository and
run `python3 release/sync_plugins.py`.
"""
    return text.encode("utf-8")


def plugin_payload() -> dict[PurePosixPath, bytes]:
    version = package_version()
    payload = source_bytes(manifest_entries())
    payload[PurePosixPath(".claude-plugin/plugin.json")] = json_bytes(plugin_manifest(version, "claude"))
    payload[PurePosixPath(".codex-plugin/plugin.json")] = json_bytes(plugin_manifest(version, "codex"))
    payload[PurePosixPath("hooks/codex.json")] = hook_payload("codex", False)
    payload[PurePosixPath("hooks/codex-windows.json")] = hook_payload("codex", True)
    payload[PurePosixPath("hooks/claude.json")] = hook_payload("claude", False)
    payload[PurePosixPath("hooks/claude-windows.json")] = hook_payload("claude", True)
    payload[PurePosixPath("README.md")] = plugin_readme()
    payload[PurePosixPath("LICENSE")] = (PROJECT_ROOT / "LICENSE").read_bytes()
    manifest = sorted([*(path.as_posix() for path in payload), "MANIFEST.txt"])
    payload[PurePosixPath("MANIFEST.txt")] = ("\n".join(manifest) + "\n").encode("utf-8")
    return payload


def expected_files() -> dict[Path, bytes]:
    plugin_files = {PLUGIN_ROOT.joinpath(*path.parts): content for path, content in plugin_payload().items()}
    market_files = {
        PROJECT_ROOT.joinpath(*path.parts): content
        for path, content in marketplace_payload().items()
    }
    return plugin_files | market_files


def actual_plugin_files() -> set[Path]:
    if not PLUGIN_ROOT.exists():
        return set()
    return {path for path in PLUGIN_ROOT.rglob("*") if path.is_file() or path.is_symlink()}


def path_uses_symlink(path: Path) -> bool:
    current = PROJECT_ROOT
    for part in path.relative_to(PROJECT_ROOT).parts:
        current /= part
        if current.is_symlink():
            return True
    return False


def check_payload(expected: dict[Path, bytes]) -> list[str]:
    errors: list[str] = []
    for path, content in expected.items():
        if path_uses_symlink(path):
            errors.append(f"symlink: {path.relative_to(PROJECT_ROOT)}")
        elif not path.is_file():
            errors.append(f"missing: {path.relative_to(PROJECT_ROOT)}")
        elif path.read_bytes() != content:
            errors.append(f"drift: {path.relative_to(PROJECT_ROOT)}")
    extras = actual_plugin_files() - {path for path in expected if PLUGIN_ROOT in path.parents}
    errors.extend(f"unexpected: {path.relative_to(PROJECT_ROOT)}" for path in sorted(extras))
    return errors


def generated_plugin_path(entry: str) -> Path:
    relative = PurePosixPath(entry)
    unsafe_part = any(part in {".", ".."} for part in relative.parts)
    if relative.is_absolute() or unsafe_part:
        raise SyncError(f"unsafe generated plugin manifest path: {entry}")
    path = PLUGIN_ROOT.joinpath(*relative.parts)
    if path_uses_symlink(path):
        raise SyncError(f"generated plugin manifest path uses a symlink: {entry}")
    return path


def prior_manifest_files() -> set[Path]:
    manifest = PLUGIN_ROOT / "MANIFEST.txt"
    if not manifest.is_file() or manifest.is_symlink():
        return set()
    entries = manifest.read_text().splitlines()
    valid_entries = entries == sorted(set(entries)) and "MANIFEST.txt" in entries
    if not valid_entries:
        raise SyncError("generated plugin manifest is not a sorted allowlist")
    return {generated_plugin_path(entry) for entry in entries}


def write_payload(expected: dict[Path, bytes]) -> None:
    symlinks = [path for path in expected if path_uses_symlink(path)]
    if symlinks:
        relative = symlinks[0].relative_to(PROJECT_ROOT)
        raise SyncError(f"generated path uses a symlink: {relative}")
    for stale in prior_manifest_files() - set(expected):
        if stale.is_file() and not stale.is_symlink():
            stale.unlink()
    for path, content in expected.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)


def run(arguments: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Generate Complexity Evaluator plugins")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args(arguments)
    try:
        expected = expected_files()
        if args.check:
            errors = check_payload(expected)
            if errors:
                print("\n".join(errors), file=sys.stderr)
                return 1
        else:
            write_payload(expected)
    except (OSError, KeyError, TypeError, ValueError, tomllib.TOMLDecodeError, SyncError) as error:
        print(f"plugin sync failed: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(run(sys.argv[1:]))
