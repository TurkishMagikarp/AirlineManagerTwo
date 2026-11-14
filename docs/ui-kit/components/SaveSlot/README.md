# SaveSlot

One saved game in the load and save list. The consumer provides the name, the game date and time, cash, fleet size and when it was written.

- Name in `body-strong`; facts in a mono meta line, in this order: game time, cash, fleet, age of the save.
- The selected slot is `accent-wash` with an `accent-text` border and carries the primary "Laden" Button.
- "Löschen" is an outline danger Button and always asks through a Dialog.
- A free slot is dashed and offers "Hier speichern".

Godot: a `Button`-like `PanelContainer` per save in a `VBoxContainer`; data from the save header only, never by loading the save.
