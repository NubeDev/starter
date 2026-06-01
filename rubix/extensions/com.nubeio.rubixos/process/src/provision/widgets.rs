//! Generate `bc_widgets` rows from a template's `widget_group` plus
//! its per-point `widget` enum (BARCODE.md §5.1 step 6, §6.3).
//!
//! A widget row says "render widget `gauge` for point `temp` on page
//! `page-floor-3` at slot N, in role `primary`". The page renderer
//! reads them and mounts the matching component — no SDUI involved,
//! so the layout is a plain serialisable table the renderer can be
//! swapped under later.
//!
//! The widget set is the template's `primary` + `secondary` keys when
//! a `widget_group` is declared; otherwise every point with a `widget`
//! gets a tile, in template order.

use starter_ext_sdk::serde_json::{json, Map, Value};
use starter_ext_sdk::Row;

use crate::provision::ids::widget_id;
use crate::provision::points::ResolvedPoint;
use crate::provision::template::DeviceTemplate;

/// Build the `bc_widgets` rows binding this device's points to a page.
pub fn rows_for(
    page_id: &str,
    device_id: &str,
    device_name: &str,
    template: &DeviceTemplate,
    points: &[ResolvedPoint],
) -> Vec<Row> {
    let ordered = ordered_keys(template, points);
    ordered
        .into_iter()
        .enumerate()
        .filter_map(|(slot, (point_key, role))| {
            let point = points.iter().find(|p| p.point_key == point_key)?;
            let widget = point.widget.clone()?;
            let title = role_title(&role, device_name, &point.source.name);
            Some(build_row(
                page_id, device_id, point, &widget, slot as i64, &role, &title,
            ))
        })
        .collect()
}

/// Decide the (point_key, role) render order. With a `widget_group`,
/// `primary` leads, then `secondary` in order; any remaining points
/// fall in afterwards as `extra` so nothing is silently dropped.
fn ordered_keys(template: &DeviceTemplate, points: &[ResolvedPoint]) -> Vec<(String, String)> {
    let mut ordered: Vec<(String, String)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    if let Some(group) = template.widget_group() {
        if let Some(primary) = &group.primary {
            ordered.push((primary.clone(), "primary".into()));
            seen.push(primary.clone());
        }
        for key in &group.secondary {
            ordered.push((key.clone(), "secondary".into()));
            seen.push(key.clone());
        }
    }
    for p in points {
        if !seen.contains(&p.point_key) {
            ordered.push((p.point_key.clone(), "extra".into()));
        }
    }
    ordered
}

fn role_title(role: &str, device_name: &str, point_name: &str) -> String {
    match role {
        "primary" => device_name.to_owned(),
        _ => point_name.to_owned(),
    }
}

fn build_row(
    page_id: &str,
    device_id: &str,
    point: &ResolvedPoint,
    widget: &str,
    slot: i64,
    role: &str,
    title: &str,
) -> Row {
    let wid = widget_id(page_id, device_id, &point.point_key, role);
    let mut row = Map::new();
    row.insert("widget_id".into(), json!(wid));
    row.insert("page_id".into(), json!(page_id));
    row.insert("device_id".into(), json!(device_id));
    row.insert("point_id".into(), json!(point.point_id));
    row.insert("widget".into(), json!(widget));
    row.insert("slot".into(), Value::from(slot));
    row.insert("role".into(), json!(role));
    row.insert("title".into(), json!(title));
    Row::from_map(row)
}
