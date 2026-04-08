//! M3U playlist writer.
//!
//! Generates valid M3U/M3U8 playlist strings from structured data.
//! Faithfully translates `@iptv/playlist`'s `writeM3U` function.

use std::collections::HashMap;

use crate::types::{M3uEntry, M3uPlaylist};

/// Write an [`M3uPlaylist`] to a valid M3U string.
///
/// # Example
///
/// ```
/// use crispy_m3u::types::{M3uEntry, M3uHeader, M3uPlaylist};
///
/// let playlist = M3uPlaylist {
///     header: M3uHeader {
///         epg_url: Some("http://epg.example.com/xmltv.xml".into()),
///         ..Default::default()
///     },
///     entries: vec![M3uEntry {
///         name: Some("CNN".into()),
///         urls: smallvec::smallvec!["http://example.com/cnn".into()],
///         tvg_id: Some("CNN.us".into()),
///         group_title: Some("News".into()),
///         duration: Some(-1.0),
///         ..Default::default()
///     }],
/// };
///
/// let output = crispy_m3u::write(&playlist);
/// assert!(output.starts_with("#EXTM3U"));
/// assert!(output.contains("CNN"));
/// ```
pub fn write(playlist: &M3uPlaylist) -> String {
    let mut out = String::with_capacity(estimate_capacity(playlist));

    // Header line.
    out.push_str("#EXTM3U");

    // EPG URL header attribute.
    if let Some(ref epg_url) = playlist.header.epg_url {
        write_attr(&mut out, "x-tvg-url", epg_url);
    }

    write_optional_attr(&mut out, "catchup", playlist.header.catchup.as_deref());
    write_optional_attr(
        &mut out,
        "catchup-days",
        playlist.header.catchup_days.as_deref(),
    );
    write_optional_attr(
        &mut out,
        "catchup-source",
        playlist.header.catchup_source.as_deref(),
    );

    // Extra header attributes.
    for (key, value) in sorted_pairs(&playlist.header.extras) {
        write_attr(&mut out, key, value);
    }

    // Channel entries.
    for entry in &playlist.entries {
        // Skip entries that have neither a URL nor retainable inline metadata.
        if !entry.should_retain() {
            continue;
        }

        out.push_str("\n#EXTINF:");

        // Duration (default -1 for live).
        match entry.duration {
            Some(d) => {
                // Write integer if it's a whole number, float otherwise.
                if d.fract() == 0.0 {
                    #[allow(clippy::cast_possible_truncation)]
                    write_int(&mut out, d as i64);
                } else {
                    out.push_str(&d.to_string());
                }
            }
            None => out.push_str("-1"),
        }

        // Known attributes.
        write_optional_attr(&mut out, "tvg-id", entry.tvg_id.as_deref());
        write_optional_attr(&mut out, "tvg-name", entry.tvg_name.as_deref());
        write_optional_attr(&mut out, "tvg-language", entry.tvg_language.as_deref());
        write_optional_attr(&mut out, "tvg-logo", entry.tvg_logo.as_deref());
        write_optional_attr(&mut out, "tvg-rec", entry.tvg_rec.as_deref());
        write_optional_attr(&mut out, "tvg-chno", entry.tvg_chno.as_deref());
        let group_title = normalized_group_title(entry);
        write_optional_attr(&mut out, "group-title", group_title.as_deref());
        write_optional_attr(&mut out, "tvg-url", entry.tvg_url.as_deref());
        write_optional_attr(&mut out, "timeshift", entry.timeshift.as_deref());
        write_optional_attr(&mut out, "catchup", entry.catchup.as_deref());
        write_optional_attr(&mut out, "catchup-days", entry.catchup_days.as_deref());
        write_optional_attr(&mut out, "catchup-source", entry.catchup_source.as_deref());

        // Radio flag.
        if entry.is_radio {
            write_attr(&mut out, "radio", "true");
        }

        // EPG time shift.
        if let Some(shift) = entry.tvg_shift {
            out.push_str(" tvg-shift=\"");
            out.push_str(&shift.to_string());
            out.push('"');
        }

        // VOD/media attributes.
        if entry.is_media {
            write_attr(&mut out, "media", "true");
        }
        write_optional_attr(&mut out, "media-dir", entry.media_dir.as_deref());
        if let Some(size) = entry.media_size {
            out.push_str(" media-size=\"");
            out.push_str(&size.to_string());
            out.push('"');
        }

        // Provider attributes.
        write_optional_attr(&mut out, "provider-name", entry.provider_name.as_deref());
        write_optional_attr(&mut out, "provider-type", entry.provider_type.as_deref());
        write_optional_attr(&mut out, "provider-logo", entry.provider_logo.as_deref());
        write_optional_attr(
            &mut out,
            "provider-countries",
            entry.provider_countries.as_deref(),
        );
        write_optional_attr(
            &mut out,
            "provider-languages",
            entry.provider_languages.as_deref(),
        );

        // Extra attributes.
        for (key, value) in sorted_pairs(&entry.extras) {
            write_attr(&mut out, key, value);
        }

        // Comma + channel name.
        out.push(',');
        if let Some(ref name) = entry.name {
            out.push_str(name);
        }

        for (key, value) in sorted_pairs(&entry.stream_properties) {
            write_directive(&mut out, "#KODIPROP:", key, value);
        }

        for (key, value) in sorted_pairs(&entry.vlc_options) {
            write_directive(&mut out, "#EXTVLCOPT:", key, value);
        }

        // Web properties (as #WEBPROP: lines).
        for (key, value) in sorted_pairs(&entry.web_properties) {
            write_directive(&mut out, "#WEBPROP:", key, value);
        }

        for url in &entry.urls {
            out.push('\n');
            out.push_str(url);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Write ` key="value"` to the output.
fn write_attr(out: &mut String, key: &str, value: &str) {
    out.push(' ');
    out.push_str(key);
    out.push_str("=\"");
    out.push_str(&escape_attr_value(value));
    out.push('"');
}

/// Write an optional attribute only if it has a value.
fn write_optional_attr(out: &mut String, key: &str, value: Option<&str>) {
    if let Some(v) = value {
        write_attr(out, key, v);
    }
}

fn write_directive(out: &mut String, prefix: &str, key: &str, value: &str) {
    out.push('\n');
    out.push_str(prefix);
    out.push_str(key);
    out.push('=');
    out.push_str(value);
}

/// Write an integer without allocating a string (itoa-style).
fn write_int(out: &mut String, n: i64) {
    // For simplicity, use `format!` which is fast enough for our purposes.
    // The itoa crate could be added for zero-alloc int formatting if needed.
    use std::fmt::Write;
    let _ = write!(out, "{n}");
}

/// Rough capacity estimate to minimize reallocations.
fn estimate_capacity(playlist: &M3uPlaylist) -> usize {
    // ~200 bytes per entry on average + header.
    200 * playlist.entries.len() + 128
}

fn escape_attr_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }

    escaped
}

fn normalized_group_title(entry: &M3uEntry) -> Option<String> {
    if !entry.groups.is_empty() {
        return Some(entry.groups.join(";"));
    }

    entry.group_title.clone()
}

fn sorted_pairs(map: &HashMap<String, String>) -> Vec<(&str, &str)> {
    let mut pairs: Vec<_> = map
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    pairs.sort_unstable_by(|(left_key, _), (right_key, _)| left_key.cmp(right_key));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{M3uEntry, M3uHeader, M3uPlaylist};
    use smallvec::smallvec;

    #[test]
    fn write_empty_playlist() {
        let playlist = M3uPlaylist::default();
        let output = write(&playlist);
        assert_eq!(output, "#EXTM3U");
    }

    #[test]
    fn write_header_with_epg_url() {
        let playlist = M3uPlaylist {
            header: M3uHeader {
                epg_url: Some("http://epg.com/guide.xml".into()),
                ..Default::default()
            },
            entries: vec![],
        };
        let output = write(&playlist);
        assert_eq!(output, r#"#EXTM3U x-tvg-url="http://epg.com/guide.xml""#);
    }

    #[test]
    fn write_header_catchup_defaults() {
        let playlist = M3uPlaylist {
            header: M3uHeader {
                catchup: Some("shift".into()),
                catchup_days: Some("7".into()),
                catchup_source: Some("http://example.com/{utc}".into()),
                ..Default::default()
            },
            entries: vec![],
        };

        let output = write(&playlist);
        assert_eq!(
            output,
            r#"#EXTM3U catchup="shift" catchup-days="7" catchup-source="http://example.com/{utc}""#
        );
    }

    #[test]
    fn write_single_channel() {
        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                name: Some("CNN".into()),
                urls: smallvec!["http://example.com/cnn".into()],
                tvg_id: Some("CNN.us".into()),
                group_title: Some("News".into()),
                duration: Some(-1.0),
                ..Default::default()
            }],
        };
        let output = write(&playlist);
        assert!(output.contains(r#"tvg-id="CNN.us""#));
        assert!(output.contains(r#"group-title="News""#));
        assert!(output.contains(",CNN\n"));
        assert!(output.contains("http://example.com/cnn"));
        assert!(output.contains("#EXTINF:-1"));
    }

    #[test]
    fn write_skips_unidentified_entries_without_url() {
        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                ..Default::default()
            }],
        };
        let output = write(&playlist);
        assert_eq!(output, "#EXTM3U");
    }

    #[test]
    fn write_preserves_identified_entries_without_url() {
        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                name: Some("No URL".into()),
                tvg_id: Some("no-url".into()),
                extras: {
                    let mut extras = std::collections::HashMap::new();
                    extras.insert("Vendor-Key".to_string(), "value".to_string());
                    extras
                },
                ..Default::default()
            }],
        };
        let output = write(&playlist);

        assert!(output.contains(r#"#EXTINF:-1 tvg-id="no-url" Vendor-Key="value",No URL"#));
        assert!(!output.contains("http://"));
    }

    #[test]
    fn write_preserves_extras_only_entries_without_url() {
        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                extras: {
                    let mut extras = std::collections::HashMap::new();
                    extras.insert("Vendor-Key".to_string(), "value".to_string());
                    extras
                },
                ..Default::default()
            }],
        };
        let output = write(&playlist);

        assert_eq!(output, "#EXTM3U\n#EXTINF:-1 Vendor-Key=\"value\",");
    }

    #[test]
    fn write_preserves_known_metadata_only_entries_without_url() {
        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                group_title: Some("News".into()),
                groups: vec!["News".into()],
                ..Default::default()
            }],
        };
        let output = write(&playlist);

        assert_eq!(output, "#EXTM3U\n#EXTINF:-1 group-title=\"News\",");
    }

    #[test]
    fn write_includes_extras() {
        let mut extras = std::collections::HashMap::new();
        extras.insert("custom".to_string(), "value".to_string());

        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                name: Some("Ch".into()),
                urls: smallvec!["http://example.com/ch".into()],
                duration: Some(-1.0),
                extras,
                ..Default::default()
            }],
        };
        let output = write(&playlist);
        assert!(output.contains(r#"custom="value""#));
    }

    #[test]
    fn write_default_duration_when_none() {
        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                name: Some("Ch".into()),
                urls: smallvec!["http://example.com/ch".into()],
                ..Default::default()
            }],
        };
        let output = write(&playlist);
        assert!(output.contains("#EXTINF:-1"));
    }

    #[test]
    fn roundtrip_parse_write_parse() {
        let original = r#"#EXTM3U x-tvg-url="http://epg.com/guide.xml"
#EXTINF:-1 tvg-id="BBC1.uk" tvg-name="BBC One" tvg-logo="http://logos.com/bbc1.png" group-title="UK",BBC One HD
http://stream.example.com/bbc1
#EXTINF:3600 tvg-id="MOV1" group-title="Movies",Test Movie
http://stream.example.com/movie1"#;

        let parsed = crate::parse(original).unwrap();
        let written = write(&parsed);
        let reparsed = crate::parse(&written).unwrap();

        assert_eq!(parsed.entries.len(), reparsed.entries.len());
        assert_eq!(parsed.header.epg_url, reparsed.header.epg_url);

        for (a, b) in parsed.entries.iter().zip(reparsed.entries.iter()) {
            assert_eq!(a.tvg_id, b.tvg_id);
            assert_eq!(a.name, b.name);
            assert_eq!(a.urls, b.urls);
            assert_eq!(a.group_title, b.group_title);
            assert_eq!(a.duration, b.duration);
            assert_eq!(a.tvg_logo, b.tvg_logo);
            assert_eq!(a.tvg_name, b.tvg_name);
        }
    }

    #[test]
    fn roundtrip_with_catchup() {
        let original = r#"#EXTM3U
#EXTINF:-1 catchup="shift" catchup-days="5" catchup-source="http://example.com/{utc}",Catchup Ch
http://example.com/stream"#;

        let parsed = crate::parse(original).unwrap();
        let written = write(&parsed);
        let reparsed = crate::parse(&written).unwrap();

        assert_eq!(reparsed.entries[0].catchup.as_deref(), Some("shift"));
        assert_eq!(reparsed.entries[0].catchup_days.as_deref(), Some("5"));
        assert_eq!(
            reparsed.entries[0].catchup_source.as_deref(),
            Some("http://example.com/{utc}")
        );
    }

    #[test]
    fn roundtrip_preserves_metadata_only_entries() {
        let original = r#"#EXTM3U
#EXTINF:-1 tvg-id="meta-only" Vendor-Key="value",Metadata Only"#;

        let parsed = crate::parse(original).unwrap();
        let written = write(&parsed);
        let reparsed = crate::parse(&written).unwrap();
        let entry = &reparsed.entries[0];

        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(reparsed.entries.len(), 1);
        assert_eq!(entry.tvg_id.as_deref(), Some("meta-only"));
        assert_eq!(entry.name.as_deref(), Some("Metadata Only"));
        assert_eq!(
            entry.extras.get("Vendor-Key").map(String::as_str),
            Some("value")
        );
        assert!(!written.contains("http://"));
    }

    #[test]
    fn roundtrip_preserves_extras_only_entries_without_url() {
        let original = r#"#EXTM3U
#EXTINF:-1 Vendor-Key="value""#;

        let parsed = crate::parse(original).unwrap();
        let written = write(&parsed);
        let reparsed = crate::parse(&written).unwrap();
        let entry = &reparsed.entries[0];

        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(reparsed.entries.len(), 1);
        assert!(entry.urls.is_empty());
        assert_eq!(
            entry.extras.get("Vendor-Key").map(String::as_str),
            Some("value")
        );
        assert_eq!(written, "#EXTM3U\n#EXTINF:-1 Vendor-Key=\"value\",");
    }

    #[test]
    fn write_preserves_multi_url_and_directive_metadata_on_roundtrip() {
        let mut stream_properties = std::collections::HashMap::new();
        stream_properties.insert(
            "inputstream.adaptive.manifest_type".to_string(),
            "hls".to_string(),
        );
        stream_properties.insert(
            "inputstream".to_string(),
            "inputstream.adaptive".to_string(),
        );

        let mut vlc_options = std::collections::HashMap::new();
        vlc_options.insert("http-user-agent".to_string(), "VLC/3.0".to_string());

        let mut web_properties = std::collections::HashMap::new();
        web_properties.insert("web-player".to_string(), "html5".to_string());

        let playlist = M3uPlaylist {
            header: M3uHeader::default(),
            entries: vec![M3uEntry {
                name: Some("Multi".into()),
                urls: smallvec![
                    "http://example.com/primary".into(),
                    "http://example.com/backup".into()
                ],
                groups: vec!["News".into(), "Local".into()],
                stream_properties,
                vlc_options,
                web_properties,
                ..Default::default()
            }],
        };

        let written = write(&playlist);
        let reparsed = crate::parse(&written).unwrap();
        let entry = &reparsed.entries[0];

        assert_eq!(
            entry.urls.as_slice(),
            &[
                "http://example.com/primary".to_string(),
                "http://example.com/backup".to_string()
            ]
        );
        assert_eq!(entry.groups, vec!["News", "Local"]);
        assert_eq!(
            entry
                .stream_properties
                .get("inputstream")
                .map(String::as_str),
            Some("inputstream.adaptive")
        );
        assert_eq!(
            entry
                .stream_properties
                .get("inputstream.adaptive.manifest_type")
                .map(String::as_str),
            Some("hls")
        );
        assert_eq!(
            entry.vlc_options.get("http-user-agent").map(String::as_str),
            Some("VLC/3.0")
        );
        assert_eq!(
            entry.web_properties.get("web-player").map(String::as_str),
            Some("html5")
        );
    }

    #[test]
    fn write_escapes_attributes_and_roundtrips_original_values() {
        let mut extras = std::collections::HashMap::new();
        extras.insert("note".to_string(), "Line 1\n\"Quoted\" \\ path".to_string());

        let playlist = M3uPlaylist {
            header: M3uHeader {
                extras: {
                    let mut header_extras = std::collections::HashMap::new();
                    header_extras.insert("description".to_string(), "Header\r\nValue".to_string());
                    header_extras
                },
                ..Default::default()
            },
            entries: vec![M3uEntry {
                name: Some("Escaped".into()),
                urls: smallvec!["http://example.com/escaped".into()],
                tvg_name: Some("Quoted \"Name\"\nLine".into()),
                extras,
                ..Default::default()
            }],
        };

        let written = write(&playlist);
        assert!(written.contains(r#"description="Header\r\nValue""#));
        assert!(written.contains(r#"tvg-name="Quoted \"Name\"\nLine""#));
        assert!(written.contains(r#"note="Line 1\n\"Quoted\" \\ path""#));

        let reparsed = crate::parse(&written).unwrap();
        let entry = &reparsed.entries[0];
        assert_eq!(
            reparsed.header.extras.get("description"),
            Some(&"Header\r\nValue".to_string())
        );
        assert_eq!(entry.tvg_name.as_deref(), Some("Quoted \"Name\"\nLine"));
        assert_eq!(
            entry.extras.get("note"),
            Some(&"Line 1\n\"Quoted\" \\ path".to_string())
        );
    }

    #[test]
    fn write_orders_map_backed_metadata_deterministically() {
        let playlist = M3uPlaylist {
            header: M3uHeader {
                extras: {
                    let mut extras = std::collections::HashMap::new();
                    extras.insert("z-last".to_string(), "2".to_string());
                    extras.insert("a-first".to_string(), "1".to_string());
                    extras
                },
                ..Default::default()
            },
            entries: vec![M3uEntry {
                name: Some("Ordered".into()),
                urls: smallvec!["http://example.com/ordered".into()],
                extras: {
                    let mut extras = std::collections::HashMap::new();
                    extras.insert("z-extra".to_string(), "2".to_string());
                    extras.insert("a-extra".to_string(), "1".to_string());
                    extras
                },
                stream_properties: {
                    let mut properties = std::collections::HashMap::new();
                    properties.insert("z-prop".to_string(), "2".to_string());
                    properties.insert("a-prop".to_string(), "1".to_string());
                    properties
                },
                vlc_options: {
                    let mut options = std::collections::HashMap::new();
                    options.insert("z-opt".to_string(), "2".to_string());
                    options.insert("a-opt".to_string(), "1".to_string());
                    options
                },
                web_properties: {
                    let mut properties = std::collections::HashMap::new();
                    properties.insert("z-web".to_string(), "2".to_string());
                    properties.insert("a-web".to_string(), "1".to_string());
                    properties
                },
                ..Default::default()
            }],
        };

        let written = write(&playlist);
        let expected = r#"#EXTM3U a-first="1" z-last="2"
#EXTINF:-1 a-extra="1" z-extra="2",Ordered
#KODIPROP:a-prop=1
#KODIPROP:z-prop=2
#EXTVLCOPT:a-opt=1
#EXTVLCOPT:z-opt=2
#WEBPROP:a-web=1
#WEBPROP:z-web=2
http://example.com/ordered"#;

        assert_eq!(written, expected);
    }
}
