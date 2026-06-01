//! Deterministic id minting for provisioning rows.
//!
//! Ids are derived, not random: re-provisioning the same device (a
//! re-scan) must reproduce the same `point_id`/`widget_id`/`alarm_id`
//! so the operation is idempotent and repairs rather than duplicates
//! (BARCODE.md §5.1, §7). A point id is `device_id:key`; widget and
//! alarm ids fold their owning keys through a small FNV-1a hash so
//! they are stable, collision-resistant within a device, and free of
//! any wall-clock or RNG input the SDK forbids.

/// Point id: `device_id` + `:` + point key.
pub fn point_id(device_id: &str, point_key: &str) -> String {
    format!("{device_id}:{point_key}")
}

/// Widget id: stable across re-scans for the same (page, device,
/// point, role) tuple.
pub fn widget_id(page_id: &str, device_id: &str, point_key: &str, role: &str) -> String {
    let h = fnv1a(&[page_id, device_id, point_key, role]);
    format!("wdg-{h:016x}")
}

/// Alarm id: stable across re-scans for the same (point, predicate).
pub fn alarm_id(point_id: &str, predicate: &str) -> String {
    let h = fnv1a(&[point_id, predicate]);
    format!("alm-{h:016x}")
}

/// Provision-log event id: derived from device + step + a monotonic
/// sequence the caller supplies, so each step in one provision logs a
/// distinct row without needing a clock.
pub fn event_id(device_id: &str, step: &str, seq: u32) -> String {
    let seq = seq.to_string();
    let h = fnv1a(&[device_id, step, &seq]);
    format!("evt-{h:016x}")
}

/// Site/location/page id from a human name plus a discriminator (its
/// kind), so the same name under two kinds doesn't collide.
pub fn slug_id(prefix: &str, name: &str) -> String {
    let slug = slugify(name);
    let h = fnv1a(&[prefix, name]);
    format!("{prefix}-{slug}-{:08x}", (h & 0xffff_ffff) as u32)
}

/// Lowercase, hyphen-separated, ASCII-only slug capped at 32 chars.
pub fn slugify(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut prev_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    trimmed.chars().take(32).collect()
}

/// FNV-1a 64-bit hash over a list of parts (joined by NUL).
fn fnv1a(parts: &[&str]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            hash ^= 0;
            hash = hash.wrapping_mul(PRIME);
        }
        for byte in part.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(PRIME);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_id_is_stable() {
        assert_eq!(point_id("DRP-1", "temp"), "DRP-1:temp");
    }

    #[test]
    fn widget_and_alarm_ids_are_deterministic() {
        let a = widget_id("page-1", "DRP-1", "temp", "primary");
        let b = widget_id("page-1", "DRP-1", "temp", "primary");
        assert_eq!(a, b);
        let c = widget_id("page-1", "DRP-1", "humidity", "primary");
        assert_ne!(a, c);
        assert_eq!(alarm_id("DRP-1:temp", "> 35"), alarm_id("DRP-1:temp", "> 35"));
        assert_ne!(alarm_id("DRP-1:temp", "> 35"), alarm_id("DRP-1:temp", "< 5"));
    }

    #[test]
    fn slugify_handles_unicode_and_spaces() {
        assert_eq!(slugify("Level 3 — North"), "level-3-north");
        assert_eq!(slugify("Building A"), "building-a");
    }

    #[test]
    fn slug_id_is_stable_per_name() {
        assert_eq!(slug_id("site", "Building A"), slug_id("site", "Building A"));
    }
}
