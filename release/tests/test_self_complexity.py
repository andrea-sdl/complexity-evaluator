import json
import os
import subprocess
import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).parents[2]
EVAL_ROOT = PROJECT_ROOT / "agent" / "eval"
EVAL_FIXTURES = EVAL_ROOT / "fixtures"
EVAL_DEPENDENCIES = PROJECT_ROOT / "agent" / "eval" / "node_modules"


def binary_covers_current_rust_sources(candidate: Path) -> bool:
    if not candidate.is_file():
        return False
    source_paths = list((PROJECT_ROOT / "src").glob("*.rs"))
    source_paths.extend((PROJECT_ROOT / name) for name in ("Cargo.toml", "Cargo.lock"))
    existing_sources = [path for path in source_paths if path.is_file()]
    if not existing_sources:
        return True
    newest_source = max(path.stat().st_mtime_ns for path in existing_sources)
    return candidate.stat().st_mtime_ns >= newest_source


def complexity_binary() -> Path:
    configured = os.environ.get("COMPLEXITY_BIN")
    candidates = []
    if configured:
        candidates.append(Path(configured).expanduser())
    for profile in ("debug", "release"):
        candidates.append(PROJECT_ROOT / "target" / profile / "complexity")
        candidates.append(PROJECT_ROOT / "target" / profile / "complexity.exe")
    for candidate in candidates:
        if binary_covers_current_rust_sources(candidate):
            return candidate.resolve()
    raise RuntimeError("build complexity or set COMPLEXITY_BIN before running this test")


def project_python_paths() -> list[Path]:
    paths = []
    for root in (PROJECT_ROOT / "agent", PROJECT_ROOT / "release"):
        paths.extend(root.rglob("*.py"))
    return sorted(
        path
        for path in paths
        if EVAL_FIXTURES not in path.parents and EVAL_DEPENDENCIES not in path.parents
    )


def eval_javascript_paths() -> list[Path]:
    return sorted(
        path
        for path in EVAL_ROOT.rglob("*.mjs")
        if EVAL_FIXTURES not in path.parents and EVAL_DEPENDENCIES not in path.parents
    )


def run_self_check(
    language: str, paths: list[Path]
) -> tuple[subprocess.CompletedProcess[str], dict]:
    relative_paths = [str(path.relative_to(PROJECT_ROOT)) for path in paths]
    result = subprocess.run(
        [
            str(complexity_binary()),
            "--language",
            language,
            "--format",
            "json",
            "--max-complexity",
            "7",
            *relative_paths,
        ],
        cwd=PROJECT_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    return result, json.loads(result.stdout)


class ProjectComplexityTests(unittest.TestCase):
    def test_project_python_code_stays_below_the_project_limits(self) -> None:
        paths = project_python_paths()
        result, report = run_self_check("python", paths)

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(report["status"], "complete")
        self.assertEqual(report["summary"]["errors"], 0)
        self.assertEqual(report["summary"]["violations"], 0)
        self.assertLessEqual(report["summary"]["max_score"], 7)
        self.assertLessEqual(report["summary"]["max_control_depth"], 3)
        self.assertLessEqual(report["summary"]["max_function_line_span"], 50)
        self.assertLessEqual(report["summary"]["max_condition_predicates"], 4)

        expected_paths = [str(path.relative_to(PROJECT_ROOT)) for path in paths]
        analyzed_paths = [file["path"] for file in report["files"]]
        self.assertEqual(analyzed_paths, expected_paths)

    def test_eval_support_code_stays_below_the_project_limits(self) -> None:
        paths = eval_javascript_paths()
        result, report = run_self_check("javascript", paths)

        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(report["status"], "complete")
        self.assertEqual(report["summary"]["errors"], 0)
        self.assertEqual(report["summary"]["violations"], 0)
        self.assertLessEqual(report["summary"]["max_score"], 7)
        self.assertLessEqual(report["summary"]["max_control_depth"], 3)
        self.assertLessEqual(report["summary"]["max_function_line_span"], 50)
        self.assertLessEqual(report["summary"]["max_condition_predicates"], 4)
        expected_paths = [str(path.relative_to(PROJECT_ROOT)) for path in paths]
        analyzed_paths = [file["path"] for file in report["files"]]
        self.assertEqual(analyzed_paths, expected_paths)


if __name__ == "__main__":
    unittest.main()
