# NavRail

The six top-level destinations, always visible at the left edge. The consumer provides the airline name, its code and base, the current destination and an attention count per destination.

- Order is fixed: Übersicht, Flotte, Routen, Karte, Finanzen, Personal.
- The current item is `accent-wash` with `accent-text`; icons inherit the label colour (1.6px stroke, 20px box).
- A count badge (`stop` fill, `on-stop` text) appears only for entries of the AttentionList that lead to that destination.
- Below 900px window width the rail collapses to icons only, 56px wide, with the label as tooltip.

Godot: `VBoxContainer` of toggle `Button`s, variation `NavButton`, icon modulated to the font colour.
