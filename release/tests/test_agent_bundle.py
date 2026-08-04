import json
import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).parents[2]
AGENT_ROOT = PROJECT_ROOT / "agent"


class AgentBundleTests(unittest.TestCase):
    def test_readme_paths_exist_from_the_archive_root(self) -> None:
        readme = (AGENT_ROOT / "README.md").read_text(encoding="utf-8")
        documented_paths = (
            "agent/hooks/codex.json",
            "agent/hooks/codex-windows.json",
            "agent/hooks/claude.json",
            "agent/hooks/claude-windows.json",
            "agent/skills/complexity-cli",
        )
        for path in documented_paths:
            self.assertIn(f"`{path}`", readme)
            self.assertTrue((PROJECT_ROOT / path).exists())

    def test_hook_samples_call_the_packaged_checker(self) -> None:
        checker = "agent/skills/complexity-cli/scripts/check_complexity.py"
        for host in ("codex", "claude"):
            for suffix, launcher in (("", "python3 "), ("-windows", "py -3 ")):
                sample = json.loads(
                    (AGENT_ROOT / "hooks" / f"{host}{suffix}.json").read_text()
                )
                for event in ("UserPromptSubmit", "Stop"):
                    command = sample["hooks"][event][0]["hooks"][0]["command"]
                    self.assertTrue(command.startswith(launcher))
                    self.assertIn(checker, command)

    def test_manifest_has_the_manual_codex_eval_and_no_legacy_runner(self) -> None:
        entries = (AGENT_ROOT / "MANIFEST.txt").read_text(encoding="utf-8").splitlines()
        required = {
            "eval/assertions/refactor-result.mjs",
            "eval/cases.yaml",
            "eval/fixtures/javascript-score/subject.js",
            "eval/fixtures/javascript-score/test.mjs",
            "eval/fixtures/php-predicates/subject.php",
            "eval/fixtures/php-predicates/test.php",
            "eval/fixtures/rust-span/subject.rs",
            "eval/fixtures/rust-span/test.rs",
            "eval/fixtures/typescript-depth/subject.ts",
            "eval/fixtures/typescript-depth/test.mjs",
            "eval/package-lock.json",
            "eval/package.json",
            "eval/promptfoo.env",
            "eval/promptfooconfig.yaml",
            "eval/scripts/eval-lib.mjs",
            "eval/scripts/run-codex-eval.mjs",
            "eval/tests/eval-contract.test.mjs",
            "eval/tests/fixtures.test.mjs",
            "eval/tests/refactor-result.test.mjs",
        }
        ignored_parts = {".promptfoo", "__pycache__", "node_modules"}
        actual = sorted(
            path.relative_to(AGENT_ROOT).as_posix()
            for path in AGENT_ROOT.rglob("*")
            if path.is_file()
            and ignored_parts.isdisjoint(path.relative_to(AGENT_ROOT).parts)
        )

        self.assertEqual(entries, sorted(set(entries)))
        self.assertEqual(entries, actual)
        self.assertTrue(required <= set(entries))
        self.assertTrue(
            {
                "eval/cases.json",
                "eval/fixtures/blocked.py",
                "eval/fixtures/fail.py",
                "eval/fixtures/pass.py",
                "eval/fixtures/revise.py",
                "eval/provider.mjs",
                "eval/run.py",
                "eval/test_run.py",
            }.isdisjoint(entries)
        )


if __name__ == "__main__":
    unittest.main()
