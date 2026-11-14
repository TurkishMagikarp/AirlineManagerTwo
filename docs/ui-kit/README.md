# Airline Manager UI Kit

The target look of the game's interface. It replaces the navy and cyan "Operations Center" theme described in `docs/ui-design.md`. The kit is a specification: the Godot implementation lives in `scripts/ui/operations_theme.gd`, `scripts/ui/components.gd` and the panels.

## How to use this folder

- `tokens.json` is the source of every colour, text style, spacing step, radius and shadow. Theme `night` is the default and the one the game implements first; `day` is a later option.
- `components/<Name>/README.md` is the guideline for one component and ends with the Godot control it maps to. Read only the components you are implementing.
- `components/<Name>/preview.html` is the visual reference. It opens directly in a browser and loads `tokens.css` and `bundle.css` from this folder. Add `data-theme="day"` on `<html>` to see the light theme. `bundle.css` holds the exact sizes, paddings and states of every component.
- `tokens.css` is generated from `tokens.json`; do not edit it.
- Sample data in the previews (airline name, prices, aircraft facts) is invented. Take values from the simulation, never from a preview.
- Fonts are Outfit, Nunito and DM Mono, all under the SIL Open Font License. The previews load them from Google Fonts; the game bundles the `.ttf` files under `resources/fonts/` with their licence texts.

## Brand book

Airline Manager looks like a modern operations desk lit by a departure board: round, open letterforms for titles and text, soft monospaced numerals for every figure, amber for the one thing that matters on each view.

### Principles

- **State is spelled out.** Every status carries a word and a glyph. Colour supports the reading and never replaces it.
- **Numbers are instruments.** Every figure is set in `mono` with tabular digits and its unit, right-aligned in columns.
- **One amber thing per view.** `accent` marks the single primary action and the current selection. If two things on a panel are accent-filled, one of them is wrong.
- **Flat and ruled.** Depth comes from four surface steps and 1px `line` rules. `shadow-pop` is reserved for what floats: menus, tooltips, dialogs.
- **Nothing moves by itself.** No decorative animation. The UI has no clock of its own; what changes on screen changes because the simulation did.

### Content

- The game ships in German and English, and every string comes from the translation table. No component contains literal text, and no layout assumes a string's length.
- Sentence case in both languages: "Flugplan bearbeiten", "Edit schedule". Uppercase appears only through the `panel-title`, `column-head`, `label` and `code` styles.
- Buttons are verb phrases that name the outcome: "Flugzeug kaufen" / "Buy aircraft", "Route löschen" / "Delete route". Never "OK", "Ja" or "Yes".
- Most text states facts: "D-ANFC wartet auf einen Techniker." / "D-ANFC is waiting for a technician." German addresses the player informally (du) and rarely.
- A missing value is an em dash, never "0" or "n/a".
- No emoji, no exclamation marks.

#### What changes with the language

| | Deutsch | English |
| --- | --- | --- |
| Money | `1.284.500 EUR` | `1,284,500 EUR` |
| Negative | `−12.480 EUR` | `−12,480 EUR` |
| Percent | `78 %` | `78%` |
| Date | `14.11.2026` | `14 Nov 2026` |
| Clock readout | `SA 14.11.2026` | `SAT 14 NOV 2026` |
| Decimal | `4,5` | `4.5` |

#### What never changes

- Currency is whole EUR with the unit after the number. Negative amounts use the true minus sign U+2212; results carry an explicit `+` when positive.
- Time is `HH:MM` in 24-hour form in both languages, as in aviation.
- ICAO codes (`EDDF`), route pairs with an en dash (`EDDF–EDDM`), registrations (`D-ANFA`) and aircraft type names are never translated. All are set in `code` style.

#### Layout for two languages

- Design every control for the longer string, which is usually the German one. Buttons, chips and tabs size to their text; nothing has a fixed width.
- Table columns take their minimum width from the longer language. Sentences in lists may shorten with an ellipsis and show the full text in a tooltip; button labels, chips and column heads never truncate.
- A status word stays within two words in both languages. If a translation needs three, pick a different word.
- `display`, `sans` and `mono` cover Western and Central European Latin. A language outside that range (Cyrillic, Greek, CJK) needs fallback faces chosen first.

### Colour

- Window background is `ground`. Panels, the rail and the clock bar are `surface`. Controls, menus and dialogs are `surface-raised`. Wells and table headers are `surface-sunken`.
- Text is `ink`; captions, labels and idle icons are `ink-muted`. `ink-faint` is for disabled text only.
- `accent` is the brand amber. It is a fill and always carries `on-accent` text. Where accent is a thin mark or text (selected tab, own route, progress fill), use `accent-text`, which darkens in the day theme to stay legible.
- `go` means a judged good result, `stop` a loss or a block, `sky` neutral information and anything airborne. Each has a `-wash` background for its chip. Do not use them as decoration.
- Caution uses the same amber (`caution` on `caution-wash`); it is told apart from selection by its triangle glyph and its word.
- The clock's `flap` cells stay dark with `flap-ink` characters in both themes.
- Two themes ship: Night ops is the default; Day ops is the same layout on light surfaces.

### Type

- `display` (Outfit) is a geometric face with round bowls: screen titles, panel titles, column heads, tabs, field labels.
- `sans` (Nunito) has rounded stroke ends and is everything that is a sentence, plus button labels.
- `mono` (DM Mono) is every number and every code. It has two weights, 400 and 500; never ask it for bold.
- Use the ten named styles as they are. Do not introduce sizes between them.

### Layout

- Spacing follows a 4px grid through `space-1` to `space-7`. Panel padding and the gap between panels are `space-4`; the content margin is `space-5`.
- Radii are small and square-shouldered: `radius-sm` for chips and flap cells, `radius-md` for controls, `radius-lg` for panels and overlays.
- Controls are 36px high, 28px in panel headers. Table rows are 40px.
- The navigation rail and clock bar stay fixed; content scrolls vertically; tables scroll inside their own frame.
- Metric cards wrap from four columns to two; side-by-side panels stack below 1100px.

### States

- Hover lifts a control one surface step. Pressed drops it to `surface-sunken`.
- Selection is `accent-wash` with `accent-text` in lists and navigation, a full `accent` fill in a segmented control.
- Keyboard focus is a solid 2px `focus` ring at 2px offset on every focusable control, drawn outside the control so it lands on the surface behind it.
- Disabled controls keep their label in `ink-faint` on `surface` and say why in a tooltip.

### Iconography

- Six navigation icons exist in the game at `resources/icons/` (overview, fleet, routes, map, finance, staff): outline, 1.6px stroke, round joins, drawn in a 20px box.
- Icons take the colour of the label beside them. The files in the repository still carry a fixed cyan stroke from the previous look and need their stroke set to white so Godot can modulate them.
- Status never uses an icon font. The five chip glyphs (circle, diamond, dash, triangle, square) are drawn shapes.

### Building it in Godot

- Every visible string goes through `tr()` with a key from the translation files (`locale/de.po`, `locale/en.po`). Numbers, money and dates are formatted by one helper that reads the active locale; `UI.money()` in `scripts/ui/components.gd` is where that starts.
- All tokens belong in one script, `scripts/ui/operations_theme.gd`, as named constants. Panels and helpers read from the theme; no `StyleBoxFlat` or colour literal is created anywhere else.
- Each component's guideline ends with the Godot control and theme variation it maps to.
- Fonts are bundled as `.ttf` files under `resources/fonts/` (all three families are under the SIL Open Font License) and assigned per theme type.

## Components

| Component | Group | Summary |
| --- | --- | --- |
| [AircraftCard](components/AircraftCard/README.md) | Data | One aircraft type on offer. The consumer provides the type name, its class, four facts and the price. |
| [AirportCard](components/AirportCard/README.md) | Map | Details of the airport selected on the map. The consumer provides the ICAO code, the city, a status and up to six facts. |
| [AttentionList](components/AttentionList/README.md) | Data | The prioritised list of things the player should deal with, above every screen. The consumer provides the entries from `SimulationClock.get_attention_snapshot()` in the order Rust returns them. |
| [Button](components/Button/README.md) | Actions | Triggers one action. The consumer provides the label as a translation key (a verb phrase in sentence case: "Flugzeug kaufen" / "Buy aircraft", never "OK") and a variant. The button sizes to its text; never give it a fixed width. |
| [Checkbox](components/Checkbox/README.md) | Inputs | Selects any number of independent options: map layers, table filters. The consumer provides the label and the state. |
| [ClockBar](components/ClockBar/README.md) | Navigation | The fixed top bar: game date and time, speed, cash and save or load. It stays outside the scrolling content. The consumer provides the clock snapshot from Rust; nothing here keeps its own time. |
| [DataTable](components/DataTable/README.md) | Data | Rows of comparable records: fleet register, routes, flight results, payroll. The consumer provides column titles, widths and rows. |
| [Dialog](components/Dialog/README.md) | Overlays | Interrupts for a decision that cannot be undone: selling an aircraft, deleting a route, overwriting a save. The consumer provides a question as title, one or two sentences of consequence and two button labels. |
| [EmptyState](components/EmptyState/README.md) | Feedback | Fills a panel or table that has nothing to show yet. The consumer provides a short title, one or two sentences and the action that creates the first entry. |
| [FinanceChart](components/FinanceChart/README.md) | Data | A line chart of money over time. The consumer provides one or two series, the period labels and the unit. |
| [KeyCap](components/KeyCap/README.md) | Settings | Shows a keyboard key or a combination. The consumer provides the key names from the input map, in the active language ("Leertaste" / "Space", "Strg" / "Ctrl"). |
| [Localization](components/Localization/README.md) | Foundations | The same parts in German and English, to check that nothing depends on one language's string lengths or number format. It is a reference, not a component. |
| [MapControls](components/MapControls/README.md) | Map | The controls that float over the map: zoom, layer toggles and the scale bar. The consumer provides the layers and the current scale. |
| [MapMarkers](components/MapMarkers/README.md) | Map | The marks drawn on the world map. The consumer provides airports, routes and aircraft positions; the map only renders them. |
| [MapScreen](components/MapScreen/README.md) | Screens | The Karte screen assembled at 1280 × 760: the map with its floating controls and a selected airport, and the routes in view below. A reference composition, not a component. |
| [MetricCard](components/MetricCard/README.md) | Data | One key figure with its caption. The consumer provides the caption, the value, its unit and at most one supporting line: a change against the previous period or a ProgressBar. |
| [NavRail](components/NavRail/README.md) | Navigation | The six top-level destinations, always visible at the left edge. The consumer provides the airline name, its code and base, the current destination and an attention count per destination. |
| [OverviewScreen](components/OverviewScreen/README.md) | Screens | The kit assembled as the Übersicht screen at 1280 × 760, to judge the parts together. It is a reference composition, not a component. |
| [Panel](components/Panel/README.md) | Layout | Groups one topic under a title. The consumer provides the title, an optional single action or status in the header, and the body. |
| [ProgressBar](components/ProgressBar/README.md) | Data | Shows how far something has come. Two forms: a bar for a share of a whole, and a route leg for a flight between two airports. The consumer provides a value from 0 to 100 and, for the leg, both ICAO codes and the arrival time. |
| [RouteCard](components/RouteCard/README.md) | Map | Details of the route selected on the map or in a table. The consumer provides both ICAO codes, the city names, a status, the current leg and up to six facts. |
| [SaveSlot](components/SaveSlot/README.md) | Data | One saved game in the load and save list. The consumer provides the name, the game date and time, cash, fleet size and when it was written. |
| [SegmentedControl](components/SegmentedControl/README.md) | Actions | Picks exactly one of two to five short options that take effect at once: game speed (Pause, 1×, 4×, 16×) and chart range. The consumer provides the options and the selected one. |
| [Select](components/Select/README.md) | Inputs | Chooses one item from a list too long for a SegmentedControl: aircraft type, airport, staff member. The consumer provides the items, each with an optional right-aligned detail in `code` style (seats, distance, price). |
| [SettingsRow](components/SettingsRow/README.md) | Settings | One setting: its name, one sentence about what it does, and one control. The consumer provides all three. |
| [SettingsScreen](components/SettingsScreen/README.md) | Screens | The settings menu assembled at 960 wide: sections on the left, Panels of SettingsRows on the right. A reference composition, not a component. |
| [Slider](components/Slider/README.md) | Inputs | Sets a value on a continuous range where the exact number matters less than the feel: volume, UI scale. The consumer provides the range, the value and a label. |
| [StatusChip](components/StatusChip/README.md) | Data | Names the state of an aircraft, route, flight or staff member in one or two words. The consumer provides the word and one of five kinds. |
| [Switch](components/Switch/README.md) | Inputs | Turns one setting on or off, effective at once. The consumer provides the state; the row it sits in provides the label. |
| [Tabs](components/Tabs/README.md) | Navigation | Switches between sibling views inside one screen, such as Register, Beschaffung and Wartungsbasen under Flotte. The consumer provides two to five labels. |
| [TextField](components/TextField/README.md) | Inputs | One line of typed input with a label above and help below. The consumer provides the label, the value, an optional unit and the help or error sentence. |
| [Toast](components/Toast/README.md) | Feedback | Confirms that something the player triggered has happened, or reports that it failed. The consumer provides a StatusChip, one sentence and at most one action. |
| [Tooltip](components/Tooltip/README.md) | Overlays | Explains a control or breaks a figure down on hover or keyboard focus. The consumer provides either one short sentence or a caption with label and value pairs. |
