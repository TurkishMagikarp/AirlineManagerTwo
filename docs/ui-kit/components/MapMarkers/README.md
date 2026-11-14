# MapMarkers

The marks drawn on the world map. The consumer provides airports, routes and aircraft positions; the map only renders them.

- Sea is `map-water` with a dotted `line` graticule every 4° of longitude and 2° of latitude; land is `map-land` with a 0.75px `line-strong` coastline. Draw coasts at a resolution where bays, estuaries and islands of 30 km and more are visible.
- The player's base is an `accent` square with its ICAO code in a tag; other airports are `ink` rings with the code beside them.
- Own routes are solid `accent-text` arcs, 2px, 3px when selected. Competitor routes are dashed `ink-faint`, so they recede.
- An aircraft in flight is an `ink` arrowhead pointing along its route.
- Codes are `code` style. Never label a route; label its two ends.

Godot: `scripts/map_*.gd` draw calls read these colours from the theme.
