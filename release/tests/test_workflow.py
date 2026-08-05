import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).parents[2]
WORKFLOW = PROJECT_ROOT / ".github" / "workflows" / "complexity-release.yml"


class ReleaseWorkflowTests(unittest.TestCase):
    def test_windows_build_runs_both_plugin_hook_samples(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        normalized_workflow = " ".join(workflow.split())

        self.assertIn(
            "if: runner.os == 'Windows' && env.RELEASE_TAG != 'complexity-v0.3.1'",
            workflow,
        )
        self.assertIn("COMPLEXITY_BIN: ${{ matrix.binary }}", workflow)
        self.assertIn(
            "py -3 -m unittest release.tests.test_plugin_package."
            "PluginPackageTests.test_each_windows_hook_sample_runs_the_packaged_checker",
            normalized_workflow,
        )

    def test_existing_tag_can_be_dispatched_with_supported_python(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        normalized_workflow = " ".join(workflow.split())
        setup_python = (
            "actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97"
        )

        self.assertIn("workflow_dispatch:", workflow)
        self.assertIn("description: Existing release tag to publish", workflow)
        self.assertIn(
            "RELEASE_TAG: ${{ inputs.release_tag || github.ref_name }}", workflow
        )
        self.assertEqual(workflow.count("ref: refs/tags/${{ env.RELEASE_TAG }}"), 2)
        self.assertEqual(workflow.count(setup_python), 2)
        self.assertEqual(workflow.count('python-version: "3.11"'), 2)
        self.assertIn("path: .release-validation", workflow)
        self.assertIn("Overlay corrected 0.3.1 tests", workflow)
        self.assertEqual(
            workflow.count("d1d6309386e095b8daaf06a83898ecfca985d909"), 2
        )
        self.assertEqual(
            workflow.count("4d4fadd951ffd468d0ad8eedde69ef04357a8439"), 1
        )
        self.assertIn(
            "cp .release-validation/agent/skills/complexity-cli/scripts/"
            "test_check_complexity.py agent/skills/complexity-cli/scripts/"
            "test_check_complexity.py",
            normalized_workflow,
        )
        self.assertNotIn("ref: ${{ github.sha }}", workflow)
        self.assertIn(
            "cargo test --release --locked --test javascript within_the_hook_budget",
            normalized_workflow,
        )
        self.assertNotIn("--skip", workflow)
        self.assertNotIn('"${{ github.ref_name }}"', workflow)


if __name__ == "__main__":
    unittest.main()
