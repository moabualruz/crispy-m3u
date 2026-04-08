//! Stable ID generation for M3U entries using the DJB2 hash algorithm.
//!
//! Faithfully translated from ynotv's `generateStableStreamId()` and
//! `stableHash()` functions in `m3u-parser.ts`.

use std::collections::HashSet;

/// DJB2 hash function.
///
/// Translated from ynotv: `hash = 5381; for c in s { hash = hash * 33 + c }`.
/// Uses wrapping arithmetic to match JavaScript's 32-bit integer overflow.
fn djb2_hash(input: &str) -> u32 {
    let mut hash: u32 = 5381;
    for c in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u32::from(c));
    }
    hash
}

/// Generate the deterministic base ID for an M3U entry.
///
/// Priority: `tvg_id` > `url` > `name`. Falls back to `"unknown"` if all
/// are `None`.
pub fn generate_stable_id_base(
    tvg_id: Option<&str>,
    url: Option<&str>,
    name: Option<&str>,
) -> String {
    if let Some(tvg_id) = tvg_id {
        let sanitized: String = tvg_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        if !sanitized.is_empty() {
            return sanitized;
        }
    }

    if let Some(url) = url {
        return format!("url_{:x}", djb2_hash(url));
    }

    if let Some(name) = name {
        return format!("name_{:x}", djb2_hash(name));
    }

    "unknown".to_string()
}

/// Resolve a deterministic base ID into a playlist-unique ID.
pub fn uniquify_stable_id(
    base: &str,
    url_seed: Option<&str>,
    seen_ids: &mut HashSet<String>,
) -> String {
    if seen_ids.insert(base.to_string()) {
        return base.to_string();
    }

    if let Some(url) = url_seed {
        let seeded = format!("{base}_{:x}", djb2_hash(url));
        if seen_ids.insert(seeded.clone()) {
            return seeded;
        }
        return resolve_collision(&seeded, seen_ids);
    }

    resolve_collision(base, seen_ids)
}

/// Generate a playlist-unique stable ID for an M3U entry.
pub fn generate_playlist_unique_id(
    tvg_id: Option<&str>,
    url: Option<&str>,
    name: Option<&str>,
    seen_ids: &mut HashSet<String>,
) -> String {
    let base = generate_stable_id_base(tvg_id, url, name);
    uniquify_stable_id(&base, url, seen_ids)
}

/// Backward-compatible wrapper for the older API name.
pub fn generate_stable_id(
    tvg_id: Option<&str>,
    url: Option<&str>,
    name: Option<&str>,
    seen_ids: &mut HashSet<String>,
) -> String {
    generate_playlist_unique_id(tvg_id, url, name, seen_ids)
}

/// Resolve a collision by appending `_1`, `_2`, etc. suffixes.
///
/// Translated from ynotv's collision handling loop.
fn resolve_collision(base: &str, seen_ids: &mut HashSet<String>) -> String {
    let mut counter = 1u32;
    loop {
        let candidate = format!("{base}_{counter}");
        if !seen_ids.contains(&candidate) {
            seen_ids.insert(candidate.clone());
            return candidate;
        }
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn djb2_hash_is_consistent() {
        let h1 = djb2_hash("hello");
        let h2 = djb2_hash("hello");
        assert_eq!(h1, h2);
        assert_ne!(djb2_hash("hello"), djb2_hash("world"));
    }

    #[test]
    fn stable_id_prefers_tvg_id() {
        let mut seen = HashSet::new();
        let id = generate_playlist_unique_id(
            Some("CNN.us"),
            Some("http://example.com/cnn"),
            Some("CNN"),
            &mut seen,
        );
        assert_eq!(id, "CNN.us");
    }

    #[test]
    fn stable_id_falls_back_to_url_hash() {
        let mut seen = HashSet::new();
        let id = generate_playlist_unique_id(
            None,
            Some("http://example.com/stream"),
            Some("My Channel"),
            &mut seen,
        );
        assert!(id.starts_with("url_"));
    }

    #[test]
    fn stable_id_falls_back_to_name_hash() {
        let mut seen = HashSet::new();
        let id = generate_playlist_unique_id(None, None, Some("My Channel"), &mut seen);
        assert!(id.starts_with("name_"));
    }

    #[test]
    fn collision_handling_appends_suffix() {
        let mut seen = HashSet::new();
        let id1 = generate_playlist_unique_id(Some("ch1"), None, None, &mut seen);
        let id2 = generate_playlist_unique_id(Some("ch1"), None, None, &mut seen);
        assert_eq!(id1, "ch1");
        assert_eq!(id2, "ch1_1");
    }

    #[test]
    fn collision_with_url_uses_url_hash_suffix() {
        let mut seen = HashSet::new();
        let id1 = generate_playlist_unique_id(
            Some("ESPN"),
            Some("http://example.com/espn1"),
            None,
            &mut seen,
        );
        let id2 = generate_playlist_unique_id(
            Some("ESPN"),
            Some("http://example.com/espn2"),
            None,
            &mut seen,
        );
        assert_eq!(id1, "ESPN");
        assert!(id2.starts_with("ESPN_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn url_hash_collision_appends_counter() {
        let mut seen = HashSet::new();
        let id1 = generate_playlist_unique_id(None, Some("http://example.com/s"), None, &mut seen);
        let id2 = generate_playlist_unique_id(None, Some("http://example.com/s"), None, &mut seen);
        assert_ne!(id1, id2);
        assert!(id2.starts_with(&format!("{id1}_")));
    }

    #[test]
    fn sanitizes_special_chars_in_tvg_id() {
        let mut seen = HashSet::new();
        let id = generate_playlist_unique_id(Some("ch@1 (HD)"), None, None, &mut seen);
        assert_eq!(id, "ch_1__HD_");
    }

    #[test]
    fn multiple_collisions_increment_counter() {
        let mut seen = HashSet::new();
        let id1 = generate_playlist_unique_id(Some("dup"), None, None, &mut seen);
        let id2 = generate_playlist_unique_id(Some("dup"), None, None, &mut seen);
        let id3 = generate_playlist_unique_id(Some("dup"), None, None, &mut seen);
        assert_eq!(id1, "dup");
        assert_eq!(id2, "dup_1");
        assert_eq!(id3, "dup_2");
    }

    #[test]
    fn stable_id_base_is_order_independent() {
        let a =
            generate_stable_id_base(Some("CNN.us"), Some("http://example.com/cnn"), Some("CNN"));
        let b =
            generate_stable_id_base(Some("CNN.us"), Some("http://example.com/cnn"), Some("CNN"));
        assert_eq!(a, b);
        assert_eq!(a, "CNN.us");
    }
}
