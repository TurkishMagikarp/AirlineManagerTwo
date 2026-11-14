# Tabs

Switches between sibling views inside one screen, such as Register, Beschaffung and Wartungsbasen under Flotte. The consumer provides two to five labels.

- Labels are `panel-title` style: condensed, uppercase, one word where possible.
- The selected tab is `ink` with a 2px `accent-text` underline; the others are `ink-muted`.
- Tabs never trigger an action and never hide unsaved input without a Dialog.

Godot: `TabBar` (not `TabContainer`'s boxed tabs); `tab_selected` is an empty box with a 2px bottom border.
