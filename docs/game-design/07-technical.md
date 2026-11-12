# Technische Grundlagen

## Verbindliche Beschlüsse

- **GDD-023 — Skalierung:** Adaptive ereignisbasierte und statistische Berechnungen bei konsistenten Simulationsregeln. Relevante individuelle Entscheidungen bleiben erhalten; Hintergrundsysteme können effizient aggregiert werden. Die angezeigte Kartenregion darf Ergebnisse nicht verändern.
- **GDD-024 — Zielplattformen (aktualisiert):** Native Desktop-Versionen für **Windows und macOS**. Keine Browseroberfläche. Die frühere Einordnung von macOS als optional ist überholt.
- **GDD-025 — Simulationszeit:** Pause, Zeitbeschleunigung und gezielte Sprünge zu Ereignissen beziehungsweise Zeitpunkten bei korrekter Ereignisreihenfolge.
- **GDD-026 — Speichern:** Mehrere Spielstände, Sicherungen, Snapshots, historische Daten und Wiederherstellung.
- **GDD-027 — Modding:** Dokumentierte, versionierte Daten- und ausgewählte Erweiterungsschnittstellen mit lokalem Mod-Manager; offline verwendbar.
- **GDD-036 — Trennung:** Eigenständig ausführbare, testbare Simulations-Engine und klar getrenntes Desktop-Frontend mit definierten Schnittstellen; kein verpflichtender Server.
- **GDD-037 — Technologieentscheidung (überarbeitet):** Rust als Simulations-Engine; die frühere Unity-Frontend-Empfehlung wurde verworfen.
- **GDD-038 — Qualitätssicherung:** Automatisierte Unit-, Integrations- und Reproduzierbarkeitstests, Konsistenzprüfungen, Performance-Benchmarks und Build-Pipelines.
- **GDD-039 — Updates:** Bei Internetverbindung prüfen und automatisch herunterladen; bei geeignetem Neustart automatisch installieren. Niemals Offline-Start verhindern oder eine aktive Sitzung unterbrechen.
- **GDD-044 — Globale Abdeckung:** Alle Kontinente sind von Beginn an zugänglich; adaptive wirtschaftliche Detailtiefe.
- **GDD-067 — Final gewählter Stack:** **Godot 4 als natives Windows-/macOS-Frontend, Rust als separate Simulations-Engine.** Das System benötigt keinen Browser und funktioniert vollständig offline.

## Verbindliche Architekturgrenzen

- Rust ist die maßgebliche Instanz für Wirtschafts-, Markt-, Personen-, Flug- und Betriebszustände.
- Godot verarbeitet Benutzereingaben und visualisiert von Rust gelieferte Zustände. Es darf keine abweichende Simulationslogik führen.
- Headless-Tests der Rust-Simulation müssen ohne Godot möglich sein.
- Spielstände müssen für die Reproduzierbarkeit notwendigen Zustand einschließlich geplanter Ereignisse und Zufallszustand sichern.
- Grafik-Performance und Detailstufen dürfen keine Geschäftsentscheidungen rückwirkend verändern.
- Installierte Desktop-Versionen müssen ohne Netzverbindung starten und spielbar bleiben.

## Noch nicht beschlossene Implementierungsdetails

- **Rust ↔ Godot:** GDExtension mit godot-rust wird für einen Prototyp **vorgeschlagen**, nicht bereits als bindende ABI-Festlegung beschlossen; Alternativen bewerten.
- **Godot-Skripte:** GDScript oder C# ist noch offen.
- **Persistenz:** Speicherformat, Schema und Migrationsstrategie offen.
- **Performance:** Hardwareprofile, konkrete Budgetgrenzen, Worker-/Parallelisierungskonzept offen.
- **Release:** macOS-Signierung und Notarisierung, Windows-Paketierung und Update-Rollback noch zu spezifizieren.
- **Mods und Datenrechte:** Sandboxing, Mod-Sicherheitsregeln und kommerzielle Nutzungsrechte der Referenzdaten sind offen.

Siehe [priorisierte offene Punkte](decisions/OPEN-QUESTIONS.md).
