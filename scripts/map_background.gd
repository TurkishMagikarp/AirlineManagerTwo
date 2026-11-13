extends Control
## Retained static draw commands: aircraft updates never rebuild the basemap.
var shapes: Array[Dictionary] = []
var draw_count := 0

func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	var geo: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://data/world_land.geojson"))
	for feature in geo["features"]:
		var geometry: Dictionary = feature["geometry"]
		var polygons: Array = geometry["coordinates"] if geometry["type"] == "MultiPolygon" else [geometry["coordinates"]]
		for polygon in polygons:
			var ring := PackedVector2Array()
			for point in polygon[0]: ring.append(Vector2(point[0], -point[1]))
			var indices := Geometry2D.triangulate_polygon(ring)
			if indices.is_empty(): continue
			var arrays: Array = []
			arrays.resize(Mesh.ARRAY_MAX)
			arrays[Mesh.ARRAY_VERTEX] = ring
			arrays[Mesh.ARRAY_INDEX] = indices
			var mesh := ArrayMesh.new()
			mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
			var bounds := Rect2(ring[0], Vector2.ZERO)
			for point in ring: bounds = bounds.expand(point)
			shapes.append({"ring": ring, "mesh": mesh, "bounds": bounds})

func _draw() -> void:
	draw_count += 1
	var camera = get_parent()
	var viewport := Rect2(Vector2.ZERO, camera.size)
	var scale: float = camera.scale_factor()
	draw_rect(viewport, Color("0b1725"))
	for lon in range(-180, 181, 30):
		for copy in range(-1, 2):
			var x := lon + copy * 360
			draw_line(camera.project(Vector2(x, -90)), camera.project(Vector2(x, 90)), Color("182b3e"))
	for lat in range(-60, 61, 30):
		draw_line(Vector2(0, camera.project(Vector2(0, lat)).y), Vector2(camera.size.x, camera.project(Vector2(0, lat)).y), Color("182b3e"))
	for copy in range(-1, 2):
		var translation: Vector2 = camera.size / 2 + camera.offset + Vector2(copy * 360 * scale, 0)
		draw_set_transform(translation, 0, Vector2(scale, scale))
		for shape in shapes:
			var bounds: Rect2 = shape["bounds"]
			if not viewport.intersects(Rect2(translation + bounds.position * scale, bounds.size * scale)): continue
			draw_mesh(shape["mesh"], null, Transform2D.IDENTITY, Color("223b4b"))
			draw_polyline(shape["ring"], Color("355466"), 1.0 / scale, true)
