# Premium UI, autonomy and accessibility

## Approved
- **GDD-020 — Visual style:** Premium hybrid of interactive map, elegant dashboards, clear charts and optional detail views.
- **GDD-021 — Control:** Delegable departments and virtual management team up to full autonomous company control, while manual takeover remains possible.
- **GDD-022 — Onboarding:** Optional contextual help, descriptions and recommendations rather than forced tutorial.
- **GDD-031 — Identity:** Full airline/brand customization including livery editor, cabin identity, uniforms, subsidiaries and imported community designs.
- **GDD-034 — BI:** Custom reporting, forecasting, scenarios, rival comparisons, causal analyses, export and automated management reports.
- **GDD-062 — Decision center:** Prioritized management proposals with costs, risks, alternatives; approve, reject, defer.
- **GDD-063 — Attention:** Priority-based, bundled, responsibility-aware notifications. Critical warnings cannot disappear silently.
- **GDD-065 — Explainability:** Layered explanation from plain summary to causes, calculations and underlying events; no invented cause text.
- **GDD-066 — Usability:** Adjustable dashboards/panels, global search, filters, shortcuts, color-blind-friendly and accessible information layouts.

## Interaction rules
The user can transition between automation and manual control at department granularity. Autonomy decisions operate on the same simulation rules, are logged and have clear authority thresholds. Visual values, explanations, forecasts and underlying ledger/state must reconcile. Colors alone must not carry status; desktop UI must remain performant on large data.

## UI asset direction (proposal, not approved)
Use native Godot Control nodes, layouts and Themes for tables/buttons/widgets; re-use a consistent licensed vector icon library; reserve generative artwork for illustrative imagery, scene decoration and carefully managed textures. Do not ship entire rendered screens as static bitmaps. Define reusable semantic design tokens, asset manifests and import validation.

## Open
Final navigation/layout grammar, interaction states, keyboard/accessibility test thresholds, concrete UI token palette, map visualization and asset licensing.
