#!/usr/bin/env python3
"""Focused tests for the complexity skill checker."""

from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("check_complexity.py")
SKILL = SCRIPT.parent.parent / "SKILL.md"
SPEC = importlib.util.spec_from_file_location("check_complexity", SCRIPT)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("cannot load check_complexity.py")
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)

FAKE_COMPLEXITY_SOURCE = """#!/usr/bin/env python3
import json
import os
import sys
from pathlib import Path

paths = [path.removeprefix("./") for path in sys.argv[5:]]
Path(os.environ["COMPLEXITY_CAPTURE"]).write_text(json.dumps(paths))
score = int(os.environ.get("COMPLEXITY_SCORE", "0"))
functions = []
if score:
    functions = [{
        "id": f"{path}:1:1",
        "name": "example",
        "score": score,
        "over_limit": score > 15,
        "signals": {
            "line_span": 3,
            "max_control_depth": 0,
            "max_condition_predicates": 0
        }
    } for path in paths]
files = [{
    "path": path,
    "status": "ok",
    "functions": [function]
} for path, function in zip(paths, functions)]
if not functions:
    files = [{"path": path, "status": "ok", "functions": []} for path in paths]
print(json.dumps({
    "schema_version": 2,
    "tool": {"name": "complexity", "version": "0.3.0"},
    "profile": "core-v1",
    "max_complexity": 15,
    "status": "complete",
    "files": files,
    "summary": {
        "files": len(files),
        "functions": len(functions),
        "violations": sum(function["over_limit"] for function in functions),
        "errors": 0
    }
}))
raise SystemExit(1 if score > 15 else 0)
"""


def fake_complexity(directory: Path) -> tuple[Path, Path]:
    binary = directory / "complexity"
    capture = directory / "paths.json"
    binary.write_text(FAKE_COMPLEXITY_SOURCE)
    binary.chmod(0o755)
    return binary, capture


def commit_all(root: Path, message: str) -> None:
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    subprocess.run(
        [
            "git",
            "-c",
            "user.name=Complexity Test",
            "-c",
            "user.email=complexity-test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            message,
        ],
        cwd=root,
        check=True,
    )


def create_merge_conflict(root: Path) -> None:
    base_branch = subprocess.run(
        ["git", "branch", "--show-current"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    subprocess.run(["git", "switch", "-q", "-c", "conflict-side"], cwd=root, check=True)
    (root / "source.ts").write_text("export const value = 1;\n")
    commit_all(root, "conflict side")
    subprocess.run(["git", "switch", "-q", base_branch], cwd=root, check=True)
    (root / "source.ts").write_text("export const value = 2;\n")
    commit_all(root, "task change")
    merge = subprocess.run(
        [
            "git",
            "-c",
            "user.name=Complexity Test",
            "-c",
            "user.email=complexity-test@example.com",
            "-c",
            "commit.gpgsign=false",
            "merge",
            "--no-edit",
            "conflict-side",
        ],
        cwd=root,
        check=False,
        capture_output=True,
    )
    if merge.returncode == 0:
        raise RuntimeError("test setup did not create a merge conflict")
    unmerged = subprocess.run(
        ["git", "diff", "--name-only", "--diff-filter=U"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.splitlines()
    if unmerged != ["source.ts"]:
        raise RuntimeError("test setup did not leave source.ts unmerged")


class HookHarness:
    def __init__(
        self,
        root: Path,
        tool_directory: Path,
        state_directory: Path,
        session_id: str,
    ) -> None:
        self.root = root
        self.session_id = session_id
        binary, self.capture = fake_complexity(tool_directory)
        self.environment = {
            **os.environ,
            "COMPLEXITY_BIN": str(binary),
            "COMPLEXITY_CAPTURE": str(self.capture),
            "COMPLEXITY_STATE_DIR": str(state_directory),
        }

    def run(self, option: str, stop_hook_active: bool = False) -> subprocess.CompletedProcess[str]:
        hook_event_name = "Stop" if option == "--hook" else "UserPromptSubmit"
        hook_input = json.dumps(
            {
                "cwd": str(self.root),
                "hook_event_name": hook_event_name,
                "session_id": self.session_id,
                "stop_hook_active": stop_hook_active,
            }
        )
        return subprocess.run(
            [sys.executable, str(SCRIPT), option],
            input=hook_input,
            check=False,
            capture_output=True,
            text=True,
            env=self.environment,
        )


def function(score: int = 0, line_span: int = 3) -> dict[str, object]:
    return {
        "id": "src/example.ts:1:1",
        "name": "example",
        "score": score,
        "over_limit": score > 15,
        "signals": {
            "line_span": line_span,
            "max_control_depth": 0,
            "max_condition_predicates": 0,
        },
    }


def report(functions: list[dict[str, object]]) -> dict[str, object]:
    violations = sum(item["over_limit"] is True for item in functions)
    return {
        "schema_version": 2,
        "tool": {"name": "complexity", "version": "0.3.0"},
        "profile": "core-v1",
        "max_complexity": 15,
        "status": "complete",
        "files": [
            {
                "path": "src/example.ts",
                "status": "ok",
                "functions": functions,
            }
        ],
        "summary": {
            "files": 1,
            "functions": len(functions),
            "violations": violations,
            "errors": 0,
        },
    }


class CheckerTests(unittest.TestCase):
    def test_pass_names_the_checked_scope(self) -> None:
        outcome, exit_code, output = CHECKER.evaluate(report([function()]), 0)

        self.assertEqual("PASS", outcome)
        self.assertEqual(0, exit_code)
        self.assertIn("CHECKED src/example.ts", output)

    def test_target_and_hard_limits_have_distinct_outcomes(self) -> None:
        revise = CHECKER.evaluate(report([function(score=11)]), 0)
        fail = CHECKER.evaluate(report([function(score=16)]), 1)

        self.assertEqual(("REVISE", 1), revise[:2])
        self.assertIn("example score=11>10", revise[2])
        self.assertEqual(("FAIL", 1), fail[:2])
        self.assertIn("example score=16>15", fail[2])

    def test_policy_finding_keeps_metric_order_and_limit_level(self) -> None:
        values = {
            "score": 16,
            "max_control_depth": 5,
            "line_span": 3,
            "max_condition_predicates": 0,
        }

        finding = CHECKER.policy_finding("src/example.ts:1:1", "example", values)

        self.assertEqual(
            (
                "FAIL",
                "src/example.ts:1:1 example score=16>15 max_control_depth=5>4",
            ),
            finding,
        )

    def test_incomplete_report_fails_closed(self) -> None:
        incomplete = report([function()])
        incomplete["status"] = "incomplete"

        with self.assertRaisesRegex(RuntimeError, "analysis is incomplete"):
            CHECKER.evaluate(incomplete, 2)

    def test_exit_and_report_mismatch_fails_closed(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "exit status.*disagree"):
            CHECKER.evaluate(report([function()]), 1)

    def test_changed_paths_include_staged_and_untracked_supported_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "staged.ts").write_text("export const staged = true;\n")
            (root / "untracked.php").write_text("<?php\n")
            (root / "staged.rs").write_text("fn staged() {}\n")
            (root / "untracked.py").write_text("def untracked():\n    pass\n")
            (root / "ignored.txt").write_text("not source\n")
            subprocess.run(
                ["git", "add", "staged.ts", "staged.rs"], cwd=root, check=True
            )

            found_root, paths = CHECKER.changed_paths(root)

            self.assertEqual(root.resolve(), found_root.resolve())
            self.assertEqual(
                ["staged.rs", "staged.ts", "untracked.php", "untracked.py"], paths
            )

    def test_stop_hook_silently_allows_unsupported_only_changes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "notes.md").write_text("No supported code changed.\n")
            hook_input = json.dumps(
                {
                    "cwd": str(root),
                    "session_id": "unsupported-only",
                    "stop_hook_active": False,
                }
            )

            result = subprocess.run(
                [sys.executable, str(SCRIPT), "--hook"],
                input=hook_input,
                check=False,
                capture_output=True,
                text=True,
            )

            self.assertEqual(0, result.returncode)
            self.assertEqual({}, json.loads(result.stdout))

    def test_stop_hook_checks_only_supported_files_from_mixed_changes(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryDirectory() as tool_directory,
            tempfile.TemporaryDirectory() as state_directory,
        ):
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "source.ts").write_text("export const value = true;\n")
            (root / "view.swift").write_text("let value = true\n")
            harness = HookHarness(
                root,
                Path(tool_directory),
                Path(state_directory),
                "mixed-files",
            )
            result = harness.run("--hook")

            self.assertEqual(0, result.returncode)
            self.assertEqual({}, json.loads(result.stdout))
            self.assertEqual(
                ["source.ts"],
                json.loads(harness.capture.read_text()),
            )

    def test_task_baseline_excludes_unchanged_preexisting_files(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryDirectory() as tool_directory,
            tempfile.TemporaryDirectory() as state_directory,
        ):
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "preexisting.ts").write_text("export const old = true;\n")
            harness = HookHarness(
                root,
                Path(tool_directory),
                Path(state_directory),
                "task-scope",
            )

            baseline = harness.run("--baseline-hook")
            self.assertEqual(0, baseline.returncode)
            self.assertEqual("", baseline.stdout)

            (root / "new.py").write_text("def new():\n    pass\n")
            stop = harness.run("--hook")

            self.assertEqual(0, stop.returncode)
            self.assertEqual({}, json.loads(stop.stdout))
            self.assertEqual(
                ["new.py"],
                json.loads(harness.capture.read_text()),
            )

    def test_task_baseline_includes_preexisting_file_changed_during_task(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryDirectory() as tool_directory,
            tempfile.TemporaryDirectory() as state_directory,
        ):
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            source = root / "preexisting.ts"
            source.write_text("export const value = 1;\n")
            harness = HookHarness(
                root,
                Path(tool_directory),
                Path(state_directory),
                "changed-preexisting",
            )
            baseline = harness.run("--baseline-hook")
            self.assertEqual(0, baseline.returncode)

            source.write_text("export const value = 2;\n")
            stop = harness.run("--hook")

            self.assertEqual(0, stop.returncode)
            self.assertEqual({}, json.loads(stop.stdout))
            self.assertEqual(
                ["preexisting.ts"],
                json.loads(harness.capture.read_text()),
            )

    def test_new_prompt_resets_a_baseline_after_a_blocked_stop(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryDirectory() as tool_directory,
            tempfile.TemporaryDirectory() as state_directory,
        ):
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            harness = HookHarness(
                root,
                Path(tool_directory),
                Path(state_directory),
                "blocked-next-task",
            )
            harness.environment["COMPLEXITY_SCORE"] = "11"
            baseline = harness.run("--baseline-hook")
            self.assertEqual(0, baseline.returncode)
            (root / "source.ts").write_text("export const value = true;\n")

            blocked = harness.run("--hook")
            self.assertEqual("block", json.loads(blocked.stdout)["decision"])

            next_prompt = harness.run("--baseline-hook")
            self.assertEqual(0, next_prompt.returncode)
            harness.capture.unlink()
            harness.environment["COMPLEXITY_SCORE"] = "0"
            (root / "new.py").write_text("def new():\n    pass\n")
            recheck = harness.run("--hook")

            self.assertEqual(0, recheck.returncode)
            self.assertEqual({}, json.loads(recheck.stdout))
            self.assertEqual(
                ["new.py"],
                json.loads(harness.capture.read_text()),
            )

    def test_new_prompt_resets_a_baseline_after_a_git_error_stop(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryDirectory() as tool_directory,
            tempfile.TemporaryDirectory() as state_directory,
        ):
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            (root / "source.ts").write_text("export const value = 0;\n")
            commit_all(root, "baseline")
            harness = HookHarness(
                root,
                Path(tool_directory),
                Path(state_directory),
                "git-error-continuation",
            )
            self.assertEqual(0, harness.run("--baseline-hook").returncode)
            create_merge_conflict(root)

            blocked = harness.run("--hook")
            self.assertEqual("block", json.loads(blocked.stdout)["decision"])
            self.assertIn("unmerged files", blocked.stdout)

            subprocess.run(["git", "merge", "--abort"], cwd=root, check=True)
            self.assertEqual(0, harness.run("--baseline-hook").returncode)
            recheck = harness.run("--hook")

            self.assertEqual({}, json.loads(recheck.stdout))
            self.assertFalse(harness.capture.exists())

    def test_task_baseline_keeps_committed_task_changes_in_scope(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryDirectory() as tool_directory,
            tempfile.TemporaryDirectory() as state_directory,
        ):
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            source = root / "source.ts"
            source.write_text("export const value = 1;\n")
            commit_all(root, "baseline")
            harness = HookHarness(
                root,
                Path(tool_directory),
                Path(state_directory),
                "committed-change",
            )
            baseline = harness.run("--baseline-hook")
            self.assertEqual(0, baseline.returncode)

            source.write_text("export const value = 2;\n")
            commit_all(root, "task change")
            stop = harness.run("--hook")

            self.assertEqual(0, stop.returncode)
            self.assertEqual({}, json.loads(stop.stdout))
            self.assertEqual(
                ["source.ts"],
                json.loads(harness.capture.read_text()),
            )

    def test_next_user_prompt_starts_a_new_task_baseline(self) -> None:
        with (
            tempfile.TemporaryDirectory() as directory,
            tempfile.TemporaryDirectory() as tool_directory,
            tempfile.TemporaryDirectory() as state_directory,
        ):
            root = Path(directory)
            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            harness = HookHarness(
                root,
                Path(tool_directory),
                Path(state_directory),
                "next-task",
            )
            harness.run("--baseline-hook")
            (root / "source.ts").write_text("export const value = true;\n")
            first_stop = harness.run("--hook")
            self.assertEqual({}, json.loads(first_stop.stdout))

            next_prompt = harness.run("--baseline-hook")
            self.assertEqual(0, next_prompt.returncode)
            harness.capture.unlink()
            next_stop = harness.run("--hook")

            self.assertEqual({}, json.loads(next_stop.stdout))
            self.assertFalse(harness.capture.exists())

    def test_repeated_stop_hook_does_not_loop(self) -> None:
        hook_input = json.dumps(
            {"cwd": str(Path.cwd()), "stop_hook_active": True}
        )
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--hook"],
            input=hook_input,
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(0, result.returncode)
        self.assertEqual({}, json.loads(result.stdout))


class SkillContractTests(unittest.TestCase):
    def test_revision_guidance_preserves_behavior_and_reports_progress(self) -> None:
        skill = SKILL.read_text()

        self.assertIn("Preserve behavior", skill)
        self.assertIn("rerun", skill)
        self.assertIn("real CLI", skill)
        self.assertIn("before/after score and metrics", skill)
        self.assertIn("next useful improvement", skill)
        self.assertIn("guard clauses", skill)
        self.assertIn("named Boolean", skill)
        self.assertIn("cohesive domain", skill)
        self.assertIn("shallow helper", skill)
        self.assertIn("exit `1` is an expected result", skill)


if __name__ == "__main__":
    unittest.main()
