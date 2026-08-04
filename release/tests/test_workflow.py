import unittest
from pathlib import Path


PROJECT_ROOT = Path(__file__).parents[2]
WORKFLOW = PROJECT_ROOT / ".github" / "workflows" / "complexity-release.yml"


class ReleaseWorkflowTests(unittest.TestCase):
    def test_existing_tag_can_be_dispatched_with_supported_python(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
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
        self.assertNotIn('"${{ github.ref_name }}"', workflow)


if __name__ == "__main__":
    unittest.main()
