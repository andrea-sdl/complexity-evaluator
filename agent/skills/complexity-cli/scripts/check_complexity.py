#!/usr/bin/env python3
"""Apply a small AI self-review policy to complexity JSON schema v2."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

SUPPORTED_SUFFIXES = {
    ".js",
    ".jsx",
    ".mjs",
    ".cjs",
    ".ts",
    ".tsx",
    ".mts",
    ".cts",
    ".php",
    ".rs",
    ".py",
}

TARGETS = {
    "score": 10,
    "max_control_depth": 3,
    "line_span": 50,
    "max_condition_predicates": 4,
}

HARD_LIMITS = {
    "score": 15,
    "max_control_depth": 4,
    "line_span": 80,
    "max_condition_predicates": 6,
}

MAX_COGNITIVE_LOAD = 2

MAX_FINDINGS = 20
MAX_FILES = 10
STATE_VERSION = 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Check JavaScript, TypeScript, PHP, Rust, and Python complexity."
    )
    parser.add_argument("paths", nargs="*", help="Files or directories to check")
    parser.add_argument(
        "--binary",
        help="Path to the complexity binary; overrides COMPLEXITY_BIN and PATH",
    )
    hook_mode = parser.add_mutually_exclusive_group()
    hook_mode.add_argument(
        "--hook",
        action="store_true",
        help="Read Stop hook input and return a hook decision",
    )
    hook_mode.add_argument(
        "--baseline-hook",
        action="store_true",
        help="Read UserPromptSubmit hook input and record the task baseline",
    )
    return parser.parse_args()


def run_git(arguments: list[str], cwd: Path) -> bytes:
    result = subprocess.run(
        ["git", *arguments],
        cwd=cwd,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        message = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(message or "Git command failed")
    return result.stdout


def git_head(cwd: Path) -> str | None:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", "HEAD"],
        cwd=cwd,
        check=False,
        capture_output=True,
    )
    if result.returncode != 0:
        return None
    try:
        return result.stdout.decode("utf-8", errors="strict").strip()
    except UnicodeDecodeError as error:
        raise RuntimeError("Git returned a non-UTF-8 commit ID") from error


def supported_paths(root: Path, raw_paths: list[bytes]) -> list[str]:
    supported: set[str] = set()
    for raw_path in raw_paths:
        if raw_path and is_supported_path(root, raw_path):
            path = decode_git_path(raw_path)
            supported.add(path)
    return sorted(supported)


def decode_git_path(raw_path: bytes) -> str:
    try:
        return raw_path.decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        raise RuntimeError("Git returned a non-UTF-8 path") from error


def is_supported_path(root: Path, raw_path: bytes) -> bool:
    path = decode_git_path(raw_path)
    return Path(path).suffix in SUPPORTED_SUFFIXES and (root / path).is_file()


def repository_root(cwd: Path) -> Path:
    try:
        root_text = run_git(["rev-parse", "--show-toplevel"], cwd)
    except RuntimeError as error:
        raise RuntimeError(
            "No paths were supplied and the current directory is not a Git repository"
        ) from error

    try:
        return Path(root_text.decode("utf-8", errors="strict").strip())
    except UnicodeDecodeError as error:
        raise RuntimeError("Git returned a non-UTF-8 repository path") from error


def changed_paths(cwd: Path) -> tuple[Path, list[str]]:
    root = repository_root(cwd)
    has_head = git_head(root) is not None

    if has_head:
        tracked = run_git(
            ["diff", "--name-only", "--diff-filter=ACMR", "-z", "HEAD", "--"],
            root,
        )
    else:
        tracked = run_git(["ls-files", "--cached", "-z"], root)

    unmerged = run_git(["diff", "--name-only", "--diff-filter=U", "-z", "--"], root)
    if unmerged.strip(b"\0"):
        raise RuntimeError("Git has unmerged files; resolve them before checking complexity")

    untracked = run_git(["ls-files", "--others", "--exclude-standard", "-z"], root)
    raw_paths = tracked.split(b"\0") + untracked.split(b"\0")

    return root, supported_paths(root, raw_paths)


def file_digest(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def state_file(root: Path, session_id: str) -> Path:
    state_root = Path(
        os.environ.get(
            "COMPLEXITY_STATE_DIR",
            Path(tempfile.gettempdir()) / "complexity-cli",
        )
    ).expanduser()
    key = hashlib.sha256(
        f"{session_id}\0{root.resolve()}".encode("utf-8")
    ).hexdigest()
    return state_root / f"{key}.json"


def write_state(path: Path, state: dict[str, Any]) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="w",
        encoding="utf-8",
        dir=path.parent,
        delete=False,
    ) as temporary:
        json.dump(state, temporary, sort_keys=True, separators=(",", ":"))
        temporary.write("\n")
        temporary_path = Path(temporary.name)
    temporary_path.chmod(0o600)
    temporary_path.replace(path)


def record_baseline(cwd: Path, session_id: str) -> None:
    root, paths = changed_paths(cwd)
    dirty = {path: file_digest(root / path) for path in paths}
    write_state(
        state_file(root, session_id),
        {
            "version": STATE_VERSION,
            "root": str(root),
            "head": git_head(root),
            "dirty": dirty,
        },
    )


def load_baseline(root: Path, session_id: str) -> dict[str, Any] | None:
    path = state_file(root, session_id)
    if not path.is_file():
        return None
    return validate_baseline_state(read_baseline_state(path), root)


def read_baseline_state(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise RuntimeError("complexity task baseline cannot be read") from error


def validate_baseline_state(state: Any, root: Path) -> dict[str, Any]:
    if not isinstance(state, dict):
        raise RuntimeError("complexity task baseline is invalid")
    validate_baseline_header(state, root)
    validate_baseline_dirty(state)
    return state


def validate_baseline_header(state: dict[str, Any], root: Path) -> None:
    if state.get("version") != STATE_VERSION:
        raise RuntimeError("complexity task baseline is invalid")
    if state.get("root") != str(root):
        raise RuntimeError("complexity task baseline is invalid")
    validate_baseline_head(state)


def validate_baseline_head(state: dict[str, Any]) -> None:
    head = state.get("head")
    if head is not None and not isinstance(head, str):
        raise RuntimeError("complexity task baseline is invalid")


def validate_baseline_dirty(state: dict[str, Any]) -> None:
    if not isinstance(state.get("dirty"), dict):
        raise RuntimeError("complexity task baseline is invalid")


def task_changed_paths(cwd: Path, session_id: str) -> tuple[Path, list[str]]:
    root, paths = changed_paths(cwd)
    baseline = load_baseline(root, session_id)
    if baseline is None:
        return root, paths

    candidates = baseline_candidates(root, paths, baseline)
    return root, changed_after_baseline(root, candidates, baseline["dirty"])


def baseline_candidates(
    root: Path,
    paths: list[str],
    baseline: dict[str, Any],
) -> set[str]:
    candidates = set(paths)
    candidates.update(committed_paths_since_baseline(root, baseline["head"]))
    return candidates


def committed_paths_since_baseline(root: Path, baseline_head: Any) -> list[str]:
    if not isinstance(baseline_head, str):
        return []
    current_head = git_head(root)
    if current_head is None or current_head == baseline_head:
        return []
    committed = run_git(
        [
            "diff",
            "--name-only",
            "--diff-filter=ACMR",
            "-z",
            baseline_head,
            current_head,
            "--",
        ],
        root,
    )
    return supported_paths(root, committed.split(b"\0"))


def changed_after_baseline(
    root: Path,
    candidates: set[str],
    dirty: Any,
) -> list[str]:
    if not isinstance(dirty, dict):
        raise RuntimeError("complexity task baseline is invalid")
    selected: list[str] = []
    for path in sorted(candidates):
        baseline_digest = dirty.get(path)
        if not isinstance(baseline_digest, str):
            selected.append(path)
            continue
        if file_digest(root / path) != baseline_digest:
            selected.append(path)
    return selected


def locate_binary(explicit: str | None) -> str:
    requested = explicit or os.environ.get("COMPLEXITY_BIN")
    if requested:
        binary = executable_path(requested)
        if binary:
            return binary
        raise RuntimeError(f"complexity binary is not executable: {requested}")

    binary = default_binary()
    if binary:
        return binary

    raise RuntimeError(
        "complexity is not on PATH; install it or set COMPLEXITY_BIN"
    )


def executable_path(requested: str) -> str | None:
    resolved = shutil.which(requested)
    if resolved:
        return resolved
    return executable_file(Path(requested).expanduser())


def executable_file(path: Path) -> str | None:
    if path.is_file() and os.access(path, os.X_OK):
        return str(path)
    return None


def default_binary() -> str | None:
    resolved = shutil.which("complexity")
    if resolved:
        return resolved
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    cargo_binary = cargo_home / "bin" / "complexity"
    return executable_file(cargo_binary)


def safe_paths(paths: list[str]) -> list[str]:
    safe: list[str] = []
    for path in paths:
        if path == "-":
            raise RuntimeError("stdin is not supported by this checker")
        if path.startswith("-"):
            safe.append(f"./{path}")
        else:
            safe.append(path)
    return safe


def run_complexity(binary: str, cwd: Path, paths: list[str]) -> tuple[int, dict[str, Any]]:
    command = [
        binary,
        "--format",
        "json",
        "--max-complexity",
        str(HARD_LIMITS["score"]),
        "--max-cognitive-load",
        str(MAX_COGNITIVE_LOAD),
        *safe_paths(paths),
    ]

    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            check=False,
            capture_output=True,
            timeout=120,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError("complexity timed out after 120 seconds") from error

    try:
        stdout = result.stdout.decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        raise RuntimeError("complexity returned non-UTF-8 output") from error

    try:
        report = json.loads(stdout)
    except json.JSONDecodeError as error:
        detail = (
            result.stderr.decode("utf-8", errors="replace").strip()
            or "complexity did not return JSON"
        )
        raise RuntimeError(detail) from error

    if not isinstance(report, dict):
        raise RuntimeError("complexity returned a non-object JSON report")
    if result.returncode not in {0, 1, 2}:
        raise RuntimeError(f"complexity exited with unexpected status {result.returncode}")

    return result.returncode, report


def required_int(value: Any, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < 0:
        raise RuntimeError(f"schema v2 field {label} is not a non-negative integer")
    return value


def evaluate(report: dict[str, Any], cli_exit: int) -> tuple[str, int, str]:
    validate_report_metadata(report, cli_exit)
    files, summary = report_records(report)
    file_count, function_count, native_violations = summary_values(summary, files)
    checked_files, findings, seen_functions, seen_native_violations = inspect_files(files)
    validate_summary_counts(
        function_count,
        native_violations,
        seen_functions,
        seen_native_violations,
    )
    cognitive_findings = readability_findings(report)
    findings.extend(cognitive_findings)
    validate_cli_exit(cli_exit, native_violations, cognitive_findings)
    return format_evaluation(
        file_count,
        function_count,
        checked_files,
        findings,
    )


def validate_report_metadata(report: dict[str, Any], cli_exit: int) -> None:
    if report.get("schema_version") != 2:
        raise RuntimeError("complexity did not return JSON schema v2")
    validate_report_tool(report.get("tool"))
    if report.get("profile") != "core-v1":
        raise RuntimeError("complexity did not use the core-v1 score profile")
    if report.get("max_complexity") != HARD_LIMITS["score"]:
        raise RuntimeError("complexity used an unexpected score limit")
    readability = report.get("readability")
    if not isinstance(readability, dict):
        raise RuntimeError("complexity did not return cognitive-load findings")
    if readability.get("max_cognitive_load") != MAX_COGNITIVE_LOAD:
        raise RuntimeError("complexity used an unexpected cognitive-load limit")
    if report.get("status") != "complete" or cli_exit == 2:
        raise RuntimeError("complexity analysis is incomplete")


def validate_report_tool(tool: Any) -> None:
    if not isinstance(tool, dict) or tool.get("name") != "complexity":
        raise RuntimeError("schema v2 report is not from complexity")


def report_records(report: dict[str, Any]) -> tuple[list[Any], dict[str, Any]]:
    files = report.get("files")
    summary = report.get("summary")
    if not isinstance(files, list) or not isinstance(summary, dict):
        raise RuntimeError("complexity schema v2 is missing files or summary")
    return files, summary


def summary_values(summary: dict[str, Any], files: list[Any]) -> tuple[int, int, int]:
    file_count = required_int(summary.get("files"), "summary.files")
    function_count = required_int(summary.get("functions"), "summary.functions")
    native_violations = required_int(
        summary.get("violations"), "summary.violations"
    )
    errors = required_int(summary.get("errors"), "summary.errors")
    if errors != 0:
        raise RuntimeError("complexity reported file errors")
    if file_count != len(files):
        raise RuntimeError("summary.files does not match the file records")
    return file_count, function_count, native_violations


def inspect_files(
    files: list[Any],
) -> tuple[list[str], list[tuple[str, str]], int, int]:
    checked_files: list[str] = []
    findings: list[tuple[str, str]] = []
    seen_functions = 0
    seen_native_violations = 0
    for file_result in files:
        path, functions = checked_file(file_result)
        checked_files.append(path)
        function_count, native_violations, file_findings = inspect_functions(functions)
        seen_functions += function_count
        seen_native_violations += native_violations
        findings.extend(file_findings)
    return checked_files, findings, seen_functions, seen_native_violations


def inspect_functions(functions: list[Any]) -> tuple[int, int, list[tuple[str, str]]]:
    findings: list[tuple[str, str]] = []
    native_violations = 0
    for function in functions:
        native_violation, finding = inspect_function(function)
        native_violations += int(native_violation)
        if finding is not None:
            findings.append(finding)
    return len(functions), native_violations, findings


def checked_file(file_result: Any) -> tuple[str, list[Any]]:
    if not isinstance(file_result, dict) or file_result.get("status") != "ok":
        raise RuntimeError("complexity reported a failed file")
    path = file_result.get("path")
    functions = file_result.get("functions")
    if not isinstance(path, str) or not isinstance(functions, list):
        raise RuntimeError("complexity schema v2 contains an invalid file record")
    return path, functions


def inspect_function(function: Any) -> tuple[bool, tuple[str, str] | None]:
    function_id, function_name, signals = function_identity(function)
    score, native_violation = function_score(function)
    values = function_values(score, signals)
    return native_violation, policy_finding(function_id, function_name, values)


def function_identity(function: Any) -> tuple[str, str, dict[str, Any]]:
    if not isinstance(function, dict):
        raise RuntimeError("complexity schema v2 contains an invalid function")
    signals = function.get("signals")
    if not isinstance(signals, dict):
        raise RuntimeError("complexity schema v2 is missing function signals")
    function_id = function.get("id")
    function_name = function.get("name")
    if not isinstance(function_id, str) or not isinstance(function_name, str):
        raise RuntimeError("complexity schema v2 is missing function identity")
    return function_id, function_name, signals


def function_score(function: dict[str, Any]) -> tuple[int, bool]:
    score = required_int(function.get("score"), "function.score")
    over_limit = function.get("over_limit")
    if not isinstance(over_limit, bool):
        raise RuntimeError("schema v2 field function.over_limit is not Boolean")
    if over_limit != (score > HARD_LIMITS["score"]):
        raise RuntimeError("function score and over_limit disagree")
    return score, over_limit


def function_values(score: int, signals: dict[str, Any]) -> dict[str, int]:
    return {
        "score": score,
        "max_control_depth": required_int(
            signals.get("max_control_depth"),
            "function.signals.max_control_depth",
        ),
        "line_span": required_int(
            signals.get("line_span"), "function.signals.line_span"
        ),
        "max_condition_predicates": required_int(
            signals.get("max_condition_predicates"),
            "function.signals.max_condition_predicates",
        ),
    }


def policy_finding(
    function_id: str,
    function_name: str,
    values: dict[str, int],
) -> tuple[str, str] | None:
    level = ""
    details: list[str] = []
    for metric, value in values.items():
        level = stricter_level(level, metric, value, details)
    if not level:
        return None
    return level, f"{function_id} {function_name} {' '.join(details)}"


def stricter_level(
    level: str,
    metric: str,
    value: int,
    details: list[str],
) -> str:
    if value > HARD_LIMITS[metric]:
        details.append(f"{metric}={value}>{HARD_LIMITS[metric]}")
        return "FAIL"
    if value > TARGETS[metric]:
        details.append(f"{metric}={value}>{TARGETS[metric]}")
        return level if level == "FAIL" else "REVISE"
    return level


def validate_summary_counts(
    function_count: int,
    native_violations: int,
    seen_functions: int,
    seen_native_violations: int,
) -> None:
    if seen_functions != function_count:
        raise RuntimeError("summary.functions does not match the function records")
    if seen_native_violations != native_violations:
        raise RuntimeError("summary.violations does not match the function records")


def readability_string(value: Any) -> str:
    if not isinstance(value, str):
        raise RuntimeError("complexity schema v2 has an invalid cognitive-load finding")
    return value


def readability_location(value: Any) -> tuple[int, int]:
    if not isinstance(value, dict):
        raise RuntimeError("complexity schema v2 has an invalid cognitive-load finding")
    line = required_int(value.get("line"), "readability.violations.location.line")
    column = required_int(
        value.get("column"), "readability.violations.location.column"
    )
    if line == 0 or column == 0:
        raise RuntimeError("complexity schema v2 has an invalid cognitive-load finding")
    return line, column


def readability_finding(violation: Any) -> tuple[str, str]:
    if not isinstance(violation, dict):
        raise RuntimeError("complexity schema v2 has an invalid cognitive-load finding")
    rule = violation.get("rule")
    if rule != "cognitive_load.inline_conditional_return":
        raise RuntimeError("complexity schema v2 has an invalid cognitive-load finding")
    readability_string(violation.get("path"))
    function_id = readability_string(violation.get("function_id"))
    readability_location(violation.get("location"))
    load = required_int(violation.get("load"), "readability.violations.load")
    if load <= MAX_COGNITIVE_LOAD:
        raise RuntimeError("complexity schema v2 has an invalid cognitive-load finding")
    return (
        "REVISE",
        f"{function_id} {rule} load={load}>{MAX_COGNITIVE_LOAD}",
    )


def readability_findings(report: dict[str, Any]) -> list[tuple[str, str]]:
    readability = report["readability"]
    violations = readability.get("violations")
    if not isinstance(violations, list):
        raise RuntimeError("complexity schema v2 has invalid cognitive-load findings")
    return [readability_finding(violation) for violation in violations]


def validate_cli_exit(
    cli_exit: int, native_violations: int, findings: list[tuple[str, str]]
) -> None:
    expected_exit = 1 if native_violations > 0 or findings else 0
    if cli_exit != expected_exit:
        raise RuntimeError("complexity exit status and schema v2 report disagree")


def format_evaluation(
    file_count: int,
    function_count: int,
    checked_files: list[str],
    findings: list[tuple[str, str]],
) -> tuple[str, int, str]:
    scope = checked_scope(checked_files)
    if not findings:
        text = (
            f"PASS complexity: {file_count} files, {function_count} functions; "
            f"all target limits met.\n{scope}"
        )
        return "PASS", 0, text
    return formatted_findings(function_count, scope, findings)


def checked_scope(checked_files: list[str]) -> str:
    scope = "CHECKED " + " ".join(checked_files[:MAX_FILES])
    if len(checked_files) > MAX_FILES:
        scope += f" ... and {len(checked_files) - MAX_FILES} more"
    return scope


def formatted_findings(
    function_count: int,
    scope: str,
    findings: list[tuple[str, str]],
) -> tuple[str, int, str]:
    outcome = "FAIL" if any(level == "FAIL" for level, _ in findings) else "REVISE"
    lines = [
        f"{outcome} complexity: {len(findings)} of {function_count} functions exceed policy.",
        scope,
    ]
    lines.extend(f"{level} {detail}" for level, detail in findings[:MAX_FINDINGS])
    if len(findings) > MAX_FINDINGS:
        lines.append(f"... {len(findings) - MAX_FINDINGS} more findings")
    return outcome, 1, "\n".join(lines)


def check(
    args: argparse.Namespace,
    session_id: str | None = None,
) -> tuple[str, int, str]:
    cwd = Path.cwd()
    paths = args.paths
    if not paths:
        if session_id is None:
            cwd, paths = changed_paths(cwd)
        else:
            cwd, paths = task_changed_paths(cwd, session_id)
        if not paths:
            return "PASS", 0, "PASS complexity: no changed supported files."

    binary = locate_binary(args.binary)
    cli_exit, report = run_complexity(binary, cwd, paths)
    return evaluate(report, cli_exit)


def hook_input() -> dict[str, Any]:
    try:
        value = json.load(sys.stdin)
    except json.JSONDecodeError as error:
        raise RuntimeError("Hook input is not valid JSON") from error
    if not isinstance(value, dict):
        raise RuntimeError("Hook input is not a JSON object")
    return value


def hook_cwd(hook: dict[str, Any]) -> Path | None:
    value = hook.get("cwd")
    if not isinstance(value, str):
        return None
    path = Path(value)
    return path if path.is_dir() else None


def hook_session_id(hook: dict[str, Any]) -> str | None:
    value = hook.get("session_id")
    return value if isinstance(value, str) and value else None


def run_baseline_hook() -> int:
    try:
        hook = hook_input()
        cwd = hook_cwd(hook)
        session_id = hook_session_id(hook)
        if cwd is not None and session_id is not None:
            record_baseline(cwd, session_id)
    except (OSError, RuntimeError):
        pass
    return 0


def checked_result(
    args: argparse.Namespace,
    session_id: str | None,
) -> tuple[int, str]:
    try:
        _, exit_code, message = check(args, session_id)
        return exit_code, message
    except (OSError, RuntimeError) as error:
        return 2, f"BLOCKED complexity: {error}"


def print_hook_result(exit_code: int, message: str) -> None:
    if exit_code == 0:
        print("{}")
        return
    print(json.dumps({"decision": "block", "reason": message}))


def run_stop_hook(args: argparse.Namespace) -> int:
    try:
        hook = hook_input()
    except RuntimeError as error:
        print_hook_result(2, f"BLOCKED complexity: {error}")
        return 0

    cwd = hook_cwd(hook)
    if cwd is not None:
        os.chdir(cwd)
    session_id = hook_session_id(hook)
    if hook.get("stop_hook_active") is True:
        print_hook_result(0, "")
        return 0

    exit_code, message = checked_result(args, session_id)
    print_hook_result(exit_code, message)
    return 0


def main() -> int:
    args = parse_args()
    if args.baseline_hook:
        return run_baseline_hook()
    if args.hook:
        return run_stop_hook(args)
    exit_code, message = checked_result(args, None)
    print(message)
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
