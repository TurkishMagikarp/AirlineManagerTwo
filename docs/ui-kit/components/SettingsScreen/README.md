# SettingsScreen

The settings menu assembled at 960 wide: sections on the left, Panels of SettingsRows on the right. A reference composition, not a component.

- Sections: Allgemein, Anzeige, Audio, Steuerung, Spielstände. The current one is `accent-wash` with `accent-text`, like the NavRail.
- Each Panel groups rows that belong together and has at most one header action.
- Language and appearance are SegmentedControls, on or off settings are Switches, ranges are Sliders.
- Opened from the ClockBar; the simulation pauses while it is open and resumes at the previous speed.
