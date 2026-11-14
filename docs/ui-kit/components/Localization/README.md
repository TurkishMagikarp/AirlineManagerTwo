# Localization

The same parts in German and English, to check that nothing depends on one language's string lengths or number format. It is a reference, not a component.

- Text comes from the translation table; the consumer passes a key, never a literal.
- Thousands separator, decimal mark, percent spacing and date order follow the language. Currency position, 24-hour time, ICAO codes and registrations do not.
- Controls size to their text. Compare the two columns: every button, chip and caption fits without truncation in both.
- The clock readout has one flap cell per character, so its width differs by language; it is left-aligned and nothing is positioned relative to its right edge.
- The language is chosen in settings with a SegmentedControl ("Deutsch", "English") and applies at once, without a restart.

Godot: `TranslationServer.set_locale()`; all labels via `tr()`, and panels re-render on `NOTIFICATION_TRANSLATION_CHANGED`.
