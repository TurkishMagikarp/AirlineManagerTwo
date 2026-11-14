# Panel

Groups one topic under a title. The consumer provides the title, an optional single action or status in the header, and the body.

- Fill `surface`, 1px `line` border, `radius-lg`, no shadow. Header 48px with a `panel-title` and a `line` divider.
- Body padding `space-4`. A table as the only content sits flush, without padding.
- Do not nest panels. Inside a panel, separate groups with a `line` divider or `space-5`.

Godot: `UI.panel(parent, title)` in `scripts/ui/components.gd` returns the body `VBoxContainer`; the header row becomes part of the helper.
