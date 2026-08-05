import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from pathlib import PurePosixPath

from release import sync_plugins


PROJECT_ROOT = Path(__file__).parents[2]
SYNC_SCRIPT = PROJECT_ROOT / "release" / "sync_plugins.py"
COMPLEXITY_BINARY = PROJECT_ROOT / "target" / "debug" / "complexity"


def runtime_binary() -> Path:
    configured = os.environ.get("COMPLEXITY_BIN")
    if configured:
        return Path(configured).expanduser().resolve()
    return COMPLEXITY_BINARY.with_suffix(".exe") if sys.platform == "win32" else COMPLEXITY_BINARY


def initialize_hook_repository(root: Path) -> None:
    subprocess.run(["git", "init", "-q"], cwd=root, check=True)
    (root / "source.js").write_text("function risky() { return true; }\n")
    subprocess.run(["git", "add", "source.js"], cwd=root, check=True)
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
            "-qm",
            "fixture",
        ],
        cwd=root,
        check=True,
    )


def hook_input(root: Path, event: str, session_id: str) -> str:
    return json.dumps(
        {"cwd": str(root), "hook_event_name": event, "session_id": session_id}
    )


def run_hook_command(
    command: str, root: Path, environment: dict[str, str], event: str, session_id: str
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=root,
        env=environment,
        input=hook_input(root, event, session_id),
        capture_output=True,
        check=False,
        shell=True,
        text=True,
    )


def exercise_hook_sample(test: unittest.TestCase, name: str) -> None:
    plugin_root = PROJECT_ROOT / "plugins/complexity-evaluator"
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        initialize_hook_repository(root)
        sample = json.loads((plugin_root / "hooks" / f"{name}.json").read_text())
        environment = {
            **os.environ,
            "CLAUDE_PLUGIN_ROOT": str(plugin_root),
            "COMPLEXITY_BIN": str(runtime_binary()),
            "COMPLEXITY_STATE_DIR": str(root / "state"),
        }
        baseline = sample["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
        baseline_result = run_hook_command(
            baseline, root, environment, "UserPromptSubmit", name
        )
        test.assertEqual(baseline_result.returncode, 0, baseline_result.stderr)
        (root / "source.js").write_text(
            "function risky(a,b,c,d,e) { if(a) { if(b) { if(c) { if(d) { if(e) return true; } } } } return false; }\n"
        )
        (root / "notes.md").write_text("unsupported change\n")
        stop = sample["hooks"]["Stop"][0]["hooks"][0]["command"]
        stop_result = run_hook_command(stop, root, environment, "Stop", name)
        test.assertEqual(stop_result.returncode, 0, stop_result.stderr)
        decision = json.loads(stop_result.stdout)
        test.assertEqual(decision["decision"], "block")
        test.assertIn("FAIL complexity", decision["reason"])
        test.assertIn("source.js", decision["reason"])
        test.assertNotIn("notes.md", decision["reason"])


class PluginPackageTests(unittest.TestCase):
    def test_generated_plugin_is_in_sync(self) -> None:
        result = subprocess.run(
            [sys.executable, SYNC_SCRIPT, "--check"],
            cwd=PROJECT_ROOT,
            capture_output=True,
            check=False,
            text=True,
        )

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_codex_manifests_follow_the_host_contract(self) -> None:
        plugin = json.loads(
            (PROJECT_ROOT / "plugins/complexity-evaluator/.codex-plugin/plugin.json").read_text()
        )
        marketplace = json.loads(
            (PROJECT_ROOT / ".agents/plugins/marketplace.json").read_text()
        )

        self.assertEqual(plugin["skills"], "./skills/")
        self.assertTrue(plugin["interface"]["longDescription"])
        self.assertEqual(
            marketplace["plugins"][0]["source"],
            {"source": "local", "path": "./plugins/complexity-evaluator"},
        )
        self.assertEqual(
            marketplace["plugins"][0]["policy"],
            {"installation": "AVAILABLE", "authentication": "ON_INSTALL"},
        )
        self.assertNotIn("version", marketplace["plugins"][0])

    def test_plugin_readme_has_install_and_opt_in_hook_steps(self) -> None:
        readme = (PROJECT_ROOT / "plugins/complexity-evaluator/README.md").read_text()

        self.assertIn(
            "codex plugin marketplace add andrea-sdl/complexity-evaluator --ref main",
            readme,
        )
        self.assertIn(
            "codex plugin add complexity-evaluator@complexity-evaluator", readme
        )
        self.assertIn(
            "claude plugin marketplace add andrea-sdl/complexity-evaluator", readme
        )
        self.assertIn(
            "claude plugin install complexity-evaluator@complexity-evaluator", readme
        )
        self.assertIn("hooks/codex.json hooks/hooks.json", readme)
        self.assertFalse((PROJECT_ROOT / "plugins/complexity-evaluator/hooks/hooks.json").exists())

    def test_project_docs_show_both_plugin_install_paths(self) -> None:
        for relative in ("README.md", "agent/README.md"):
            readme = (PROJECT_ROOT / relative).read_text()
            self.assertIn(
                "codex plugin add complexity-evaluator@complexity-evaluator", readme
            )
            self.assertIn(
                "claude plugin install complexity-evaluator@complexity-evaluator", readme
            )
            self.assertNotIn("not yet an installable", readme)

    @unittest.skipIf(sys.platform == "win32", "Windows test runners may forbid symlinks")
    def test_sync_check_rejects_a_symlinked_plugin_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            real_plugin = project_root / "outside-plugin"
            real_plugin.mkdir()
            (real_plugin / "README.md").write_bytes(b"expected\n")
            plugin_root = project_root / "plugins" / "complexity-evaluator"
            plugin_root.parent.mkdir()
            plugin_root.symlink_to(real_plugin, target_is_directory=True)
            expected = {plugin_root / "README.md": b"expected\n"}
            original_project_root = sync_plugins.PROJECT_ROOT
            original_plugin_root = sync_plugins.PLUGIN_ROOT
            sync_plugins.PROJECT_ROOT = project_root
            sync_plugins.PLUGIN_ROOT = plugin_root
            try:
                errors = sync_plugins.check_payload(expected)
            finally:
                sync_plugins.PROJECT_ROOT = original_project_root
                sync_plugins.PLUGIN_ROOT = original_plugin_root

        self.assertTrue(any(error.startswith("symlink:") for error in errors), errors)

    def test_sync_never_removes_a_path_named_outside_the_plugin(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            plugin_root = project_root / "plugins" / "complexity-evaluator"
            plugin_root.mkdir(parents=True)
            outside = project_root / "outside.txt"
            outside.write_text("keep me\n")
            (plugin_root / "MANIFEST.txt").write_text("../../outside.txt\n")
            expected = {plugin_root / "MANIFEST.txt": b"MANIFEST.txt\n"}
            original_project_root = sync_plugins.PROJECT_ROOT
            original_plugin_root = sync_plugins.PLUGIN_ROOT
            sync_plugins.PROJECT_ROOT = project_root
            sync_plugins.PLUGIN_ROOT = plugin_root
            try:
                with self.assertRaises(sync_plugins.SyncError):
                    sync_plugins.write_payload(expected)
            finally:
                sync_plugins.PROJECT_ROOT = original_project_root
                sync_plugins.PLUGIN_ROOT = original_plugin_root

            self.assertTrue(outside.is_file())

    def test_sync_rejects_an_agent_manifest_path_outside_the_source_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            agent_root = project_root / "agent"
            agent_root.mkdir()
            (agent_root / "MANIFEST.txt").write_text(
                "skills/../../outside.txt\n",
                encoding="utf-8",
            )
            original_agent_root = sync_plugins.AGENT_ROOT
            sync_plugins.AGENT_ROOT = agent_root
            try:
                with self.assertRaisesRegex(sync_plugins.SyncError, "unsafe agent manifest path"):
                    sync_plugins.manifest_entries()
            finally:
                sync_plugins.AGENT_ROOT = original_agent_root

    @unittest.skipIf(sys.platform == "win32", "Windows test runners may forbid symlinks")
    def test_sync_rejects_a_symlinked_agent_source_parent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            project_root = Path(temporary_directory)
            agent_root = project_root / "agent"
            outside = project_root / "outside"
            outside.mkdir()
            (outside / "SKILL.md").write_text("outside\n", encoding="utf-8")
            (agent_root / "skills").mkdir(parents=True)
            (agent_root / "skills" / "escape").symlink_to(
                outside,
                target_is_directory=True,
            )
            original_agent_root = sync_plugins.AGENT_ROOT
            sync_plugins.AGENT_ROOT = agent_root
            try:
                with self.assertRaisesRegex(sync_plugins.SyncError, "uses a symlink"):
                    sync_plugins.source_bytes(
                        [PurePosixPath("skills/escape/SKILL.md")]
                    )
            finally:
                sync_plugins.AGENT_ROOT = original_agent_root

    def test_plugin_manifest_is_the_exact_sorted_file_allowlist(self) -> None:
        plugin_root = PROJECT_ROOT / "plugins/complexity-evaluator"
        entries = (plugin_root / "MANIFEST.txt").read_text().splitlines()
        actual = sorted(
            path.relative_to(plugin_root).as_posix()
            for path in plugin_root.rglob("*")
            if path.is_file()
        )

        self.assertEqual(entries, sorted(set(entries)))
        self.assertEqual(entries, actual)

    def test_plugin_copies_only_the_canonical_skill_and_eval_files(self) -> None:
        agent_root = PROJECT_ROOT / "agent"
        plugin_root = PROJECT_ROOT / "plugins/complexity-evaluator"
        source_entries = (agent_root / "MANIFEST.txt").read_text().splitlines()
        copied_entries = [
            entry for entry in source_entries if entry.startswith(("skills/", "eval/"))
        ]

        for entry in copied_entries:
            self.assertEqual((plugin_root / entry).read_bytes(), (agent_root / entry).read_bytes())

    def test_plugin_is_explicit_only_and_hook_samples_are_opt_in(self) -> None:
        plugin_root = PROJECT_ROOT / "plugins/complexity-evaluator"
        skill = (plugin_root / "skills/complexity-cli/SKILL.md").read_text()
        codex_metadata = (
            plugin_root / "skills/complexity-cli/agents/openai.yaml"
        ).read_text()

        self.assertIn("disable-model-invocation: true", skill)
        self.assertIn("allow_implicit_invocation: false", codex_metadata)
        self.assertFalse((plugin_root / "hooks/hooks.json").exists())
        for name in ("codex", "codex-windows", "claude", "claude-windows"):
            sample = json.loads((plugin_root / "hooks" / f"{name}.json").read_text())
            self.assertNotIn("skillOverrides", sample)
            for event in ("UserPromptSubmit", "Stop"):
                command = sample["hooks"][event][0]["hooks"][0]["command"]
                self.assertIn("CLAUDE_PLUGIN_ROOT", command)
                self.assertNotIn("agent/skills", command)
                if "windows" in name:
                    self.assertIn(r"\skills\complexity-cli\scripts", command)
                    self.assertNotIn(r"\\skills", command)

    @unittest.skipIf(sys.platform == "win32", "POSIX hook commands use python3")
    def test_each_posix_hook_sample_runs_the_packaged_checker(self) -> None:
        self.assertTrue(runtime_binary().is_file())
        for host in ("codex", "claude"):
            with self.subTest(host=host):
                exercise_hook_sample(self, host)

    @unittest.skipUnless(sys.platform == "win32", "Windows samples need py -3")
    def test_each_windows_hook_sample_runs_the_packaged_checker(self) -> None:
        self.assertTrue(runtime_binary().is_file())
        for host in ("codex-windows", "claude-windows"):
            with self.subTest(host=host):
                exercise_hook_sample(self, host)


if __name__ == "__main__":
    unittest.main()
