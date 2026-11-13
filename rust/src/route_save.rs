//! Public route groups on disk; the operations engine keeps one service per plane.
//! Keeping service IDs stable preserves maintenance resumes and lifetime archives.
use crate::routes::Route;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use serde_json::Value;

pub fn serialize<S: Serializer>(routes: &[Route], serializer: S) -> Result<S::Ok, S::Error> {
    let mut groups = Vec::new();
    for root in routes.iter().filter(|r| r.parent_route_id.is_none()) {
        let mut value = serde_json::to_value(root).map_err(serde::ser::Error::custom)?;
        let object = value.as_object_mut().unwrap();
        object.remove("aircraft_id");
        object.remove("parent_route_id");
        let services: Vec<_> = routes.iter().filter(|r| r.public_id() == root.id).collect();
        object.insert(
            "aircraft_ids".into(),
            serde_json::json!(
                services
                    .iter()
                    .filter_map(|r| r.aircraft_id)
                    .collect::<Vec<_>>()
            ),
        );
        object.insert(
            "rotations".into(),
            serde_json::to_value(
                services
                    .iter()
                    .filter(|r| r.id != root.id)
                    .collect::<Vec<_>>(),
            )
            .map_err(serde::ser::Error::custom)?,
        );
        groups.push(value);
    }
    groups.serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Route>, D::Error> {
    let groups = Vec::<Value>::deserialize(deserializer)?;
    let mut routes = Vec::new();
    for mut value in groups {
        let object = value
            .as_object_mut()
            .ok_or_else(|| D::Error::custom("Route fehlt"))?;
        if object.contains_key("aircraft_id") || object.contains_key("parent_route_id") {
            return Err(D::Error::custom("Veraltete Routenzuweisung"));
        }
        let ids: Vec<u32> = serde_json::from_value(
            object
                .remove("aircraft_ids")
                .ok_or_else(|| D::Error::custom("Flugzeugliste fehlt"))?,
        )
        .map_err(D::Error::custom)?;
        let mut rotations: Vec<Route> = serde_json::from_value(
            object
                .remove("rotations")
                .ok_or_else(|| D::Error::custom("Rotationen fehlen"))?,
        )
        .map_err(D::Error::custom)?;
        let root_id: u32 = serde_json::from_value(object.get("id").cloned().unwrap_or(Value::Null))
            .map_err(D::Error::custom)?;
        let child_ids: Vec<_> = rotations.iter().filter_map(|r| r.aircraft_id).collect();
        let primary: Vec<_> = ids
            .iter()
            .filter(|id| !child_ids.contains(id))
            .copied()
            .collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        if primary.len() > 1
            || unique.len() != ids.len()
            || child_ids.iter().any(|id| !ids.contains(id))
            || rotations.iter().any(|r| r.parent_route_id != Some(root_id))
        {
            return Err(D::Error::custom("Widersprüchliche Flugzeugliste"));
        }
        object.insert("aircraft_id".into(), serde_json::json!(primary.first()));
        let root: Route = serde_json::from_value(value).map_err(D::Error::custom)?;
        routes.push(root);
        routes.append(&mut rotations);
    }
    routes.sort_by_key(|r| r.id);
    Ok(routes)
}

/// v14 and earlier had one aircraft per route. Also migrate AI companies, which
/// use the same grouped format even though they still operate single services.
pub fn migrate(state: &mut Value) {
    fn collection(value: &mut Value) {
        if let Some(routes) = value.as_array_mut() {
            for route in routes {
                if let Some(object) = route.as_object_mut()
                    && let Some(id) = object.remove("aircraft_id")
                {
                    object.insert(
                        "aircraft_ids".into(),
                        if id.is_null() {
                            serde_json::json!([])
                        } else {
                            serde_json::json!([id])
                        },
                    );
                    object.insert("rotations".into(), serde_json::json!([]));
                }
            }
        }
    }
    if let Some(routes) = state.get_mut("routes").and_then(|r| r.get_mut("routes")) {
        collection(routes);
    }
    if let Some(competitors) = state.get_mut("competitors").and_then(Value::as_array_mut) {
        for c in competitors {
            if let Some(routes) = c.get_mut("game").and_then(|g| g.get_mut("routes")) {
                collection(routes);
            }
        }
    }
}
