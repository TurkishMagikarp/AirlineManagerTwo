import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("reconcile", ROOT / "tools/paperclip/reconcile.py")
delivery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(delivery)

HEAD = "a" * 40
PR = {"state": "open", "draft": False, "number": 7, "head": {"sha": HEAD}}
ISSUE = {"identifier": "AIR-398", "status": "in_review", "assigneeAgentId": "developer"}
PASSING = {name: "success" for name in delivery.CHECKS}
QUEUE = [{"issue": "AIR-398", "decision": "GDD-047", "resume": True}]


class DeliveryTest(unittest.TestCase):
    def decision(self, **overrides):
        args = dict(issue=ISSUE, pr=PR, jobs=PASSING, artifact=True, files=["rust/src/aircraft.rs"],
                    approved=False, protection=True, history={}, now=10000)
        return delivery.decide(**(args | overrides))[0]

    def test_merge_requires_all_current_ci_and_branch_protection(self):
        self.assertEqual(self.decision(), "merge")
        self.assertEqual(self.decision(jobs={"rust": "success"}), "wait")
        self.assertEqual(self.decision(artifact=False), "wait")
        self.assertEqual(self.decision(protection=False), "wait")
        self.assertEqual(self.decision(files=[]), "wait")

    def test_failed_ci_reuses_existing_issue_then_stops_at_retry_limit(self):
        failed = PASSING | {"godot": "failure"}
        self.assertEqual(self.decision(jobs=failed), "wake")
        self.assertEqual(self.decision(jobs=failed, history={"attempts": 1, "last_wake": 9950}), "wait")
        self.assertEqual(self.decision(jobs=failed, history={"attempts": delivery.MAX_RETRIES}), "block")
        self.assertEqual(self.decision(pr=PR | {"state": "closed"}), "block")

    def test_no_merge_from_draft_policy_or_stale_screenshot_notes(self):
        self.assertEqual(self.decision(pr=PR | {"draft": True}), "wake")
        self.assertEqual(self.decision(files=["docs/game-design/02-world.md"]), "wait")
        self.assertEqual(self.decision(files=["docs/game-design/02-world.md"], approved=True), "merge")
        evidence = f"UI evidence {HEAD}: route-1440x1000.png route-680x520.png — no labels clipped"
        self.assertEqual(self.decision(files=["scripts/routes_panel.gd"], pr=PR | {"body": evidence}), "merge")
        self.assertEqual(self.decision(files=["scripts/routes_panel.gd"],
                                       pr=PR | {"body": evidence.replace(HEAD, "b" * 40)}), "wake")

    def test_merged_pr_is_source_of_truth_after_restart(self):
        self.assertEqual(self.decision(pr=PR | {"state": "closed", "merged_at": "2026-11-19"}), "done")
        self.assertEqual(self.decision(issue=ISSUE | {"status": "done"}), "wait")
        self.assertEqual(self.decision(issue=ISSUE | {"status": "blocked"}), "wait")

    def test_backlog_resume_is_one_time_and_never_runs_two_issues(self):
        blocked = ISSUE | {"status": "blocked"}
        self.assertEqual(delivery.choose(QUEUE, [blocked], "developer", "planner", {}), (blocked, QUEUE[0]))
        self.assertEqual(delivery.choose(QUEUE, [blocked], "developer", "planner",
                                         {"AIR-398": {"resumed": True}}), (None, None))
        self.assertEqual(delivery.choose(QUEUE, [blocked, ISSUE | {"identifier": "AIR-500"}],
                                         "developer", "planner", {})[0]["identifier"], "AIR-500")
        with self.assertRaises(RuntimeError):
            delivery.choose(QUEUE, [ISSUE, ISSUE | {"identifier": "AIR-500"}], "developer", "planner", {})

    def test_notes_task_must_come_from_planner(self):
        candidate = ISSUE | {"status": "backlog", "createdAt": "2026-11-19",
                             "description": f"Delivery source: {HEAD}"}
        self.assertEqual(delivery.choose([], [candidate], "developer", "planner", {}), (None, None))
        candidate["createdByAgentId"] = "planner"
        self.assertEqual(delivery.choose([], [candidate], "developer", "planner", {}), (candidate, None))

    def test_restores_only_the_matching_board_recovery(self):
        controller = object.__new__(delivery.Controller)
        controller.cfg = {"developer": "developer"}
        controller.pc = Mock()
        controller.intent = Mock(return_value=True)
        controller.now = 42
        blocked = ISSUE | {"id": "issue-id", "status": "blocked"}
        action = {"id": "recovery-id", "kind": "stranded_assigned_issue", "returnOwnerAgentId": "developer"}
        controller.pc.call.return_value = {"active": action}
        history = {}
        controller.promote(blocked, history)
        path, method, body = controller.pc.call.call_args.args
        self.assertEqual((path, method), ("/issues/issue-id/recovery-actions/resolve", "POST"))
        self.assertEqual((body["actionId"], body["sourceIssueStatus"]), ("recovery-id", "todo"))
        self.assertTrue(history["resumed"])
        controller.pc.call.reset_mock()
        controller.pc.call.return_value = {"active": action | {"returnOwnerAgentId": "another-agent"}}
        with self.assertRaises(RuntimeError):
            controller.promote(blocked, {})
        controller.pc.call.assert_called_once_with("/issues/issue-id/recovery-actions")
