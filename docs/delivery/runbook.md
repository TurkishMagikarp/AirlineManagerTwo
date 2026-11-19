# Unbeaufsichtigte Entwicklung mit Paperclip

`docs/game-design/decisions/DECISION-REGISTER.md` definiert die beschlossenen Spielregeln. Vorschläge und offene Fragen sind keine Aufträge. Umsetzung, interne Architektur und reversible technische Entscheidungen sind delegiert. Neue oder geänderte Produktregeln brauchen eine ausdrückliche Freigabe des Owners.

## Aufträge

- `backlog.json` ist nach Priorität sortiert. Neue Einträge bekommen einen eindeutigen `key`, einen freigegebenen `decision`-Verweis (`GDD-NNN`), `title` und überprüfbare `acceptance`. `issue` verweist auf ein bestehendes Paperclip-Ticket; `resume: true` ermöglicht einmalig die Wiederaufnahme eines bekannten Blocks.
- Für eine freigegebene, noch nicht geschnittene Anforderung kann der Owner eine Notiz in `docs/delivery/requests/` ablegen. Die Notiz nennt die beschlossene GDD-Regel und das gewünschte Verhalten. Nach einem neuen Notiz-Commit erhält Planner einmalig einen Abgleichauftrag, sobald die priorisierte Liste frei ist.
- Der Controller startet jeweils nur ein Developer-Ticket. Developer nutzt beim Wiederanlauf denselben Worktree, Branch und PR; volle Prüfungen laufen in GitHub Actions. Ungeklärte Produktentscheidungen werden geparkt, nicht für andere Aufträge vererbt.

## Betrieb

`tools/paperclip/reconcile.py` führt einen Abgleich als One-shot aus. Konfiguration und Status liegen außerhalb des Git-Checkouts. `--mode observe` liest nur. `--mode work` verwaltet Tickets ohne Merge. `--mode merge` verlangt die konfigurierte Bot-Identität, vier erfolgreiche Jobs auf dem aktuellen PR-Commit, ein CI-Screenshot-Artefakt, ggf. UI-Nachweis und einen aktiven GitHub-Regelsatz. Änderungen an Design und Automatisierung benötigen zusätzlich ein Owner-Review des aktuellen Commits. Ein Merge wird erst im folgenden Abgleich als abgeschlossen erfasst.

Der Prozess benötigt die IDs für `company`, `project`, `developer` und `planner`, `repo` (`owner/repo`), `owner`, `paperclip_api`, Dateipfade `paperclip_token`, `github_token` sowie `github_bot_login`. Der optionale `initial_notes_revision` verhindert einen rückwirkenden Planungsauftrag für den bestehenden Dokumentstand. Die private GitHub App wird nur für dieses Repo installiert: Contents und Pull requests Read/Write, Actions Read; kein Adminrecht und keine Regelsatz-Ausnahme. In GitHub `main` durch einen strikten Regelsatz mit `rust`, `godot`, `windows` und `macos` aus GitHub Actions schützen.

`github_token.py` erneuert das auf das einzelne Repo beschränkte Installationstoken. Es liest eine separate Root-Konfiguration (`app_id`, `installation_id`, `private_key`, `repo`, `github_token`, `token_staging`, `agent_uid`, `agent_gid`). Der Schlüssel und das Staging-Verzeichnis bleiben außerhalb der Container-Mounts. Nur das kurzlebige Token wird an Developer durchgereicht. Private Schlüssel nie in Chat, Repo, Agent-Instruktionen oder Logs einfügen.

Auf dem Server soll nur der neue Reconciler den Ablauf steuern. Alte Planungsroutine und alter Airline-Watchdog bleiben deaktiviert. Zum Pausieren den Reconciler-Timer anhalten. Bei einem Ausfall zunächst `observe` ausführen und GitHub- sowie Paperclip-Status vergleichen. Statusdatei nicht löschen, um Wiederholungsgrenzen nicht zurückzusetzen. Arbeit nicht durch einen neuen Ticket-Schlüssel oder entfernten Worktree „reparieren“.

## Abnahme

1. Drei beschlossene Aufträge werden ohne manuelle Freigabe normaler Features ausgeliefert; jeder hat genau ein Ticket, einen PR und einen bestätigten Merge.
2. Ein fehlgeschlagener CI-Job lässt das Ticket offen und weckt Developer für denselben PR. Nach dem Retry-Limit gibt es einen Block mit Ursache, keine Duplikate.
3. Nach Paperclip-Neustart bleibt der Worktree erhalten; der Abgleich setzt den Ablauf fort, ohne den vorhandenen PR oder die Aufgabe zu duplizieren.
