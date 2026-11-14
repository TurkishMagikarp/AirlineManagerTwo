# Slider

Sets a value on a continuous range where the exact number matters less than the feel: volume, UI scale. The consumer provides the range, the value and a label.

- Track 4px in `surface-sunken` with a `line-strong` edge, filled part `accent-text`, thumb an `ink` bar.
- The current value is always printed to the right in `data` style with its unit.
- Money, prices and counts are never a slider; they are a numeric TextField.

Godot: `HSlider` with `slider`, `grabber_area` and `grabber` themed, beside a value `Label`.
