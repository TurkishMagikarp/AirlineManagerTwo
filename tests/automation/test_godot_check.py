import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.name == "posix" and shutil.which("timeout"), "Requires the Linux shell harness")
class GodotCheckTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / "tools").mkdir()
        (self.root / "tests").mkdir()
        (self.root / "tests/sample_test.gd").touch()
        self.runner = self.root / "tools/godot_check.sh"
        shutil.copy(ROOT / "tools/godot_check.sh", self.runner)
        self.executable("cargo", "exit 0")
        self.executable("Xvfb", "exit 0")
        self.executable("godot", """
for arg in "$@"; do
    if [[ "$arg" == --import ]]; then exit "${TEST_IMPORT_STATUS:-0}"; fi
    if [[ "$arg" == --capture-dir=* ]]; then touch "${arg#*=}/partial.png"; fi
done
echo 'sample_test: PASS'
exit "${TEST_GODOT_STATUS:-0}"
""")

    def executable(self, name, content):
        path = self.root / name
        path.write_text("#!/usr/bin/env bash\n" + content + "\n")
        path.chmod(0o755)

    def run_check(self, *args, **overrides):
        env = os.environ | {
            "PATH": f"{self.root}:{os.environ['PATH']}",
            "AIRLINE_GODOT": str(self.root / "godot"),
        } | overrides
        return subprocess.run(["bash", self.runner, *args], env=env,
                              capture_output=True, text=True, timeout=10)

    def test_accepts_named_pass_marker(self):
        result = self.run_check("test", "sample")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("sample_test: PASS", result.stdout)

    def test_import_failure_is_not_a_test_pass(self):
        result = self.run_check("test", "sample", TEST_IMPORT_STATUS="7")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("FAIL Godot import", result.stderr)

    def test_nonzero_renderer_cannot_pass_with_an_image(self):
        output = self.root / "shots"
        result = self.run_check("shots", str(output), TEST_GODOT_STATUS="9")
        self.assertTrue((output / "partial.png").exists())
        self.assertIn("FAIL screenshot capture", result.stderr)
        self.assertNotEqual(result.returncode, 0)
        stale = self.run_check("shots", str(output))
        self.assertIn("must not contain existing PNGs", stale.stderr)
        self.assertNotEqual(stale.returncode, 0)
