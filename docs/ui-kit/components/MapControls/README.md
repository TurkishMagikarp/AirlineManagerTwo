# MapControls

The controls that float over the map: zoom, layer toggles and the scale bar. The consumer provides the layers and the current scale.

- Zoom is a vertical group of 36px buttons: in, out, centre on base. It sits top right.
- Layers is a small raised panel of Checkboxes, top left. Each carries the swatch of the mark it shows.
- The scale bar sits bottom left on the map itself, without a panel, in `ink`.
- All floating controls are `surface-raised` with `shadow-pop`, `space-4` from the map edge.

Godot: `Control`s anchored to the map's corners, drawn above the map canvas.
