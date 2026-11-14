# Select

Chooses one item from a list too long for a SegmentedControl: aircraft type, airport, staff member. The consumer provides the items, each with an optional right-aligned detail in `code` style (seats, distance, price).

- The closed control looks like a default Button with a chevron; the menu is `surface-raised` with `shadow-pop` and `radius-lg`.
- The current item is marked with `accent-wash` and `accent-text`; the hovered item with `line`.
- Sort by what the player decides on (price, distance), not alphabetically, when a detail column is shown.

Godot: `OptionButton` and its `PopupMenu`; style `panel`, `hover` and the item separation on `PopupMenu`.
