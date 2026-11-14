# Button

Triggers one action. The consumer provides the label as a translation key (a verb phrase in sentence case: "Flugzeug kaufen" / "Buy aircraft", never "OK") and a variant. The button sizes to its text; never give it a fixed width.

- **Primary** (`accent` fill, `on-accent` label): the one action a panel exists for. At most one per panel.
- **Default** (`surface-raised`, `line-strong` border): everything else.
- **Danger** (`stop` outline): opens a destructive flow such as deleting a route. The filled `stop` variant appears only as the confirming button inside a Dialog.
- **Ghost**: the way out of a flow ("Abbrechen", "Laden") beside a stronger button.
- Height 36px, or 28px (`--sm`) inside a panel header or table row. Radius `radius-md`.
- Disabled keeps its label readable in `ink-faint` and explains why in a tooltip.

Godot: `Button` with theme variations `PrimaryButton`, `DangerButton`, `GhostButton`; one `StyleBoxFlat` each for normal, hover, pressed, disabled, plus a transparent 2px `focus` box in the `focus` colour.
