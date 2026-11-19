"""AirlineManager's single Paperclip/GitHub reconciliation tick (stdlib only)."""

import argparse
import base64
from datetime import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import time
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen

TERMINAL = {"done", "cancelled"}
CHECKS = {"rust", "godot", "windows", "macos"}
SHA = r"[0-9a-f]{40}"
MAX_RETRIES = 3
COOLDOWN = 8 * 60
SENSITIVE = (".github/", "tools/", "docs/game-design/", "docs/delivery/",
             "AGENTS.md", "CLAUDE.md")
UI_PATHS = ("scripts/", "scenes/", "resources/", "project.godot", "rust/src/lib.rs")


def decide(issue, pr, jobs, artifact, files, approved, protection, history, now):
    """Return one action from observed state; absence of evidence never permits merge."""
    if pr and pr.get("merged_at"):
        return ("done", "") if issue["status"] not in TERMINAL else ("wait", "")
    if issue["status"] in TERMINAL:
        return "wait", ""
    if issue["status"] == "blocked":
        return "wait", ""
    if pr and pr["state"] != "open":
        return "block", "The linked PR was closed without merging."
    if pr and pr.get("mergeable_state") in {"behind", "dirty"}:
        reason = f"PR #{pr['number']} is {pr['mergeable_state']}; update the same branch against main."
    elif not pr or pr["draft"] or issue["status"] != "in_review":
        reason = "Resume the existing issue and worktree; finish the draft PR and put the issue in review."
    elif not CHECKS <= jobs.keys() or any(jobs[key] is None for key in CHECKS):
        return "wait", "Waiting for all four CI jobs on the current commit."
    elif failed := sorted(key for key in CHECKS if jobs[key] != "success"):
        reason = f"CI failed on PR #{pr['number']} ({', '.join(failed)}). Fix it on the same branch."
    elif not artifact:
        return "wait", "Current CI run has no screenshot artifact."
    elif not files:
        return "wait", "Could not verify PR files."
    elif any(f.startswith(SENSITIVE) for f in files) and not approved:
        return "wait", "Current commit changes protected policy; owner approval is required."
    elif any(f.startswith(UI_PATHS) for f in files) and not ui_evidence(pr):
        reason = "Update the PR with screenshot evidence or the UI-unchanged explanation for its current SHA."
    elif not protection:
        return "wait", "GitHub does not enforce the required checks on main."
    else:
        return "merge", ""
    if now - history.get("last_wake", 0) < COOLDOWN:
        return "wait", "Recovery cooldown."
    if history.get("attempts", 0) >= MAX_RETRIES:
        return "block", f"Recovery limit reached. Last problem: {reason}"
    return "wake", reason


def ui_evidence(pr):
    head = re.escape(pr["head"]["sha"])
    text = pr.get("body") or ""
    return bool(re.search(rf"^UI unchanged {head}: \S.{{12,}}$", text, re.M)
                or re.search(rf"^UI evidence {head}: \S+-1440x1000\.png \S+-680x520\.png \S.{{8,}}$", text, re.M))


def matches(entry, issue):
    if entry.get("issue"):
        return issue["identifier"] == entry["issue"]
    return f"Delivery key: {entry['key']}" in (issue.get("description") or "").splitlines()


def choose(queue, issues, developer, planner, history):
    """Never jump to a second feature while any Developer issue is active."""
    active = [i for i in issues if i.get("assigneeAgentId") == developer
              and i["status"] in {"todo", "in_progress", "in_review"}]
    if len(active) > 1:
        raise RuntimeError("Multiple active Developer issues; reconcile assignments before continuing.")
    if active:
        return active[0], None
    for entry in queue:
        issue = next((i for i in issues if matches(entry, i)), None)
        if issue is None:
            if "issue" in entry:
                raise RuntimeError(f"Backlog refers to missing issue {entry['issue']}")
            return None, entry
        status = issue["status"]
        if status == "backlog" or (status == "blocked" and entry.get("resume") is True
                                    and not history.get(issue["identifier"], {}).get("resumed")):
            return issue, entry
    planned = [i for i in issues if i.get("createdByAgentId") == planner
               and i.get("assigneeAgentId") == developer and i["status"] == "backlog"
               and re.search(rf"^Delivery source: {SHA}$", i.get("description") or "", re.M)]
    return (min(planned, key=lambda i: i["createdAt"]), None) if planned else (None, None)


class API:
    def __init__(self, base, token, github=False):
        self.base = base
        self.headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
        if github:
            self.headers.update({"Accept": "application/vnd.github+json",
                                 "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "airline-delivery"})

    def call(self, path, method="GET", data=None, headers=None):
        payload = None if data is None else json.dumps(data).encode()
        request = Request(self.base + path, payload, self.headers | (headers or {}), method=method)
        try:
            with urlopen(request, timeout=25) as response:
                result = response.read()
                return json.loads(result) if result else None
        except HTTPError as error:
            # Do not log body/headers: either service may return private metadata.
            raise RuntimeError(f"{method} {path.split('?')[0]} failed: HTTP {error.code}") from None


def age(timestamp, now):
    return now - datetime.fromisoformat(timestamp.replace("Z", "+00:00")).timestamp()


def validate_queue(queue):
    keys = set()
    for entry in queue:
        if ("issue" in entry) == ("key" in entry):
            raise ValueError("Each queue entry needs exactly one of issue or key.")
        key = entry.get("issue") or entry["key"]
        if key in keys or not re.fullmatch(r"[A-Za-z0-9-]{3,64}", key):
            raise ValueError(f"Invalid or duplicate queue key: {key}")
        if not re.fullmatch(r"GDD-\d{3}( / GDD-\d{3})*", entry["decision"]):
            raise ValueError(f"Invalid approved decision reference for {key}")
        keys.add(key)
    return queue


class Controller:
    def __init__(self, config, state, mode):
        self.cfg, self.state, self.mode = config, state, mode
        self.pc = API(config["paperclip_api"], Path(config["paperclip_token"]).read_text().strip())
        self.gh = API("https://api.github.com", Path(config["github_token"]).read_text().strip(), True)
        self.repo = f"/repos/{config['repo']}"
        self.company = f"/companies/{config['company']}"
        self.now = time.time()

    def patch(self, issue, status, **fields):
        self.pc.call(f"/issues/{issue['id']}", "PATCH", {"status": status, **fields})

    def comment(self, issue, text):
        path = f"/issues/{issue['id']}/comments"
        if not any(c.get("body") == text for c in self.pc.call(path)):
            self.pc.call(path, "POST", {"body": text})

    def intent(self, text):
        print(f"{self.mode}: {text}", flush=True)
        return self.mode != "observe"

    def create(self, key, title, description, assignee):
        if self.intent(f"Create {title}"):
            idem = hashlib.sha256(f"airline-delivery:{key}".encode()).hexdigest()
            self.pc.call(self.company + "/issues", "POST", {
                "title": title, "description": description, "status": "todo",
                "assigneeAgentId": assignee, "projectId": self.cfg["project"],
            }, {"Idempotency-Key": idem})

    def notes(self, issues):
        # Trigger on a new approved note revision, never on a timer alone.
        note_paths = ("docs/game-design", "docs/delivery/requests")
        changes = []
        for path in note_paths:
            query = urlencode({"sha": "main", "path": path, "per_page": 1})
            changes += self.gh.call(f"{self.repo}/commits?{query}")
        if not changes:
            return
        latest = max(changes, key=lambda c: c["commit"]["committer"]["date"])
        sha = latest["sha"]
        if sha == self.cfg.get("initial_notes_revision"):
            return
        marker = f"Notes revision: {sha}"
        if any(marker in (i.get("description") or "").splitlines() for i in issues):
            return
        self.create("notes-" + sha, "Reconcile approved repository notes",
                    f"{marker}\nRead approved notes at this revision and create at most one approved Developer issue.",
                    self.cfg["planner"])

    def verified_pr(self, issue):
        pattern = re.compile(rf"pc/{re.escape(issue['identifier'].lower())}-[a-z0-9-]+$")
        def candidates(path):
            return [pr for pr in self.gh.call(path) if pattern.fullmatch(pr["head"]["ref"])
                    and (pr["head"].get("repo") or {}).get("full_name", "").lower() == self.cfg["repo"].lower()]
        path = self.repo + "/pulls?base=main&per_page=100"
        opened = candidates(path + "&state=open")
        if len(opened) > 1:
            raise RuntimeError(f"Multiple open PRs for {issue['identifier']}")
        if opened:
            return self.gh.call(f"{self.repo}/pulls/{opened[0]['number']}")
        closed = candidates(path + "&state=closed&sort=updated&direction=desc")
        return next((p for p in closed if p.get("merged_at")), None) or (closed[0] if closed else None)

    def checks(self, pr):
        query = urlencode({"head_sha": pr["head"]["sha"], "event": "pull_request", "per_page": 25})
        runs = self.gh.call(f"{self.repo}/actions/workflows/ci.yml/runs?{query}")["workflow_runs"]
        run = next((r for r in runs if r["head_sha"] == pr["head"]["sha"]), None)
        if not run:
            return {}, False
        path = f"{self.repo}/actions/runs/{run['id']}"
        jobs = self.gh.call(f"{path}/attempts/{run['run_attempt']}/jobs")["jobs"]
        statuses = {j["name"]: j["conclusion"] for j in jobs if j["head_sha"] == pr["head"]["sha"]}
        artifact = any(a["name"] == "godot-screenshots" and not a["expired"]
                       for a in self.gh.call(f"{path}/artifacts")["artifacts"])
        return statuses, artifact

    def gates(self, pr):
        path = f"{self.repo}/pulls/{pr['number']}"
        # Fail closed if a feature exceeds one page of changed files.
        files = [f["filename"] for f in self.gh.call(f"{path}/files?per_page=100")]
        if len(files) != pr.get("changed_files"):
            files = []
        approvals = self.gh.call(f"{path}/reviews?per_page=100") if any(f.startswith(SENSITIVE) for f in files) else []
        latest = next((r for r in reversed(approvals)
                       if r["user"]["login"].lower() == self.cfg["owner"].lower()
                       and r["state"] in {"APPROVED", "CHANGES_REQUESTED", "DISMISSED"}), {})
        approved = latest.get("state") == "APPROVED" and latest.get("commit_id") == pr["head"]["sha"]
        rules = self.gh.call(f"{self.repo}/rules/branches/main")
        protected = any(r["type"] == "required_status_checks"
                        and r["parameters"]["strict_required_status_checks_policy"]
                        and CHECKS <= {c["context"] for c in r["parameters"]["required_status_checks"]
                                       if c.get("integration_id") == 15368}
                        for r in rules)
        return files, approved, protected

    def tick(self):
        if self.pc.call(self.company + "/live-runs"):
            return
        blob = self.gh.call(self.repo + "/contents/docs/delivery/backlog.json?ref=main")
        queue = validate_queue(json.loads(base64.b64decode(blob["content"])))
        issues = self.pc.call(self.company + "/issues")
        active_planning = [i for i in issues if i.get("assigneeAgentId") == self.cfg["planner"]
                           and i["title"] == "Reconcile approved repository notes"
                           and i["status"] in {"todo", "in_progress"}]
        issue, entry = choose(queue, issues, self.cfg["developer"], self.cfg["planner"], self.state)
        if not issue and entry:
            self.create(entry["key"], entry["title"],
                        f"Delivery key: {entry['key']}\nApproved decision: {entry['decision']}\n\n{entry['acceptance']}",
                        self.cfg["developer"])
            return
        if not issue:
            if active_planning:
                planning = active_planning[0]
                self.recover(planning, "Finish reconciling the approved notes.", self.cfg["planner"])
            else:
                self.notes(issues)
            return
        ident = issue["identifier"]
        history = self.state.setdefault(ident, {})
        pr = self.verified_pr(issue)
        if pr and pr.get("merged_at"):
            if self.intent(f"{ident} done: confirmed {pr['html_url']} merged"):
                self.comment(issue, f"Confirmed merged: {pr['html_url']}")
                self.patch(issue, "done")
            return
        if issue["status"] in {"backlog", "blocked"}:
            if self.intent(f"{ident} ready: resume in existing worktree"):
                self.patch(issue, "todo", assigneeAgentId=self.cfg["developer"])
                history.update(resumed=True, last_wake=self.now)
            return
        jobs, artifact, files, approved, protected = {}, False, [], False, False
        if pr and pr["state"] == "open" and not pr["draft"] and issue["status"] == "in_review":
            jobs, artifact = self.checks(pr)
            if CHECKS <= jobs.keys() and all(jobs[key] == "success" for key in CHECKS):
                files, approved, protected = self.gates(pr)
        action, reason = decide(issue, pr, jobs, artifact, files, approved, protected, history, self.now)
        if action == "merge":
            if self.mode != "merge":
                print(f"{self.mode}: {ident} is eligible to merge; no merge issued.", flush=True)
                return
            expected = self.cfg.get("github_bot_login")
            if not expected or pr["user"]["login"] != expected:
                raise RuntimeError("Merge identity or PR author is not the configured dedicated bot.")
            installation = self.gh.call("/installation/repositories")
            if installation["total_count"] != 1 or installation["repositories"][0]["full_name"].lower() != self.cfg["repo"].lower():
                raise RuntimeError("GitHub credential must be an App installation token scoped to this repository.")
            self.intent(f"{ident} merge PR #{pr['number']}")
            result = self.gh.call(f"{self.repo}/pulls/{pr['number']}/merge", "PUT",
                                  {"sha": pr["head"]["sha"], "merge_method": "merge"})
            if not result["merged"]:
                raise RuntimeError(f"GitHub did not merge PR #{pr['number']}")
            # The next tick independently verifies the merge before closing Paperclip work.
        elif action == "wake":
            self.recover(issue, reason, self.cfg["developer"])
        elif action == "block" and self.intent(f"{ident} blocked: {reason}"):
            self.comment(issue, reason)
            self.patch(issue, "blocked")
        elif reason:
            print(f"{ident}: {reason}", flush=True)

    def recover(self, issue, reason, agent):
        ident = issue["identifier"]
        history = self.state.setdefault(ident, {})
        if self.now - history.get("last_wake", 0) < COOLDOWN or age(issue["updatedAt"], self.now) < COOLDOWN:
            return
        attempts = history.get("attempts", 0)
        if attempts >= MAX_RETRIES:
            if self.intent(f"{ident} blocked: recovery exhausted"):
                self.comment(issue, f"Recovery limit reached. Last problem: {reason}")
                self.patch(issue, "blocked")
            return
        if self.intent(f"{ident} recovery {attempts + 1}: {reason}"):
            self.pc.call(f"/agents/{agent}/wakeup", "POST", {
                "source": "automation", "triggerDetail": "system", "reason": reason,
                "payload": {"issueId": issue["id"]}, "forceFreshSession": True,
                "idempotencyKey": f"airline-delivery:{issue['id']}:{attempts + 1}",
            })
            history.update(attempts=attempts + 1, last_wake=self.now)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--mode", choices=("observe", "work", "merge"), default="observe")
    args = parser.parse_args()
    state = json.loads(args.state.read_text()) if args.state.exists() else {}
    Controller(json.loads(args.config.read_text()), state, args.mode).tick()
    if args.mode != "observe":
        # The systemd oneshot serializes ticks; rename prevents partial JSON on interruption.
        temp = args.state.with_suffix(".tmp")
        temp.write_text(json.dumps(state, indent=2) + "\n")
        os.replace(temp, args.state)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError) as error:
        print(f"Reconciliation stopped: {error}", file=sys.stderr)
        sys.exit(1)
