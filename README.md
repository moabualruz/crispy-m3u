# crispy-m3u

High-performance M3U and M3U8 playlist parser/writer for IPTV workflows.

## What This Crate Is

`crispy-m3u` parses `#EXTM3U` playlists into structured Rust types and writes them back out. It is intended for IPTV applications, playlist import pipelines, migration tools, and cleanup utilities.

It focuses on the metadata commonly seen in real IPTV playlists, not just the minimal M3U format.

## What It Provides

- `parse(&str) -> Result<M3uPlaylist, M3uError>`
- `parse_with_mode(&str, ParseMode) -> Result<M3uPlaylist, M3uError>`
- `write(&M3uPlaylist) -> String`
- deterministic stable-ID base generation plus playlist-unique collision resolution
- support for common IPTV metadata such as:
  - `tvg-id`
  - `tvg-name`
  - `tvg-logo`
  - `group-title`
  - catchup fields
  - provider-specific extra attributes
- multi-URL entries and directive-backed metadata (`#KODIPROP`, `#EXTVLCOPT`, `#WEBPROP`)
- deterministic serialization for map-backed metadata
- safe attribute escaping for quotes, backslashes, and line breaks
- case-insensitive matching for known attributes while preserving original key spelling for unknown extras

## Installation

```toml
[dependencies]
crispy-m3u = "0.1.1"
```

MSRV: Rust `1.85`

## Quick Start

```rust
use crispy_m3u::{parse, write};

let input = "#EXTM3U\n#EXTINF:-1 tvg-id=\"cnn\" group-title=\"News\",CNN\nhttp://example.com/live/cnn.m3u8\n";
let playlist = parse(input).unwrap();

assert_eq!(playlist.entries.len(), 1);
assert_eq!(playlist.entries[0].name.as_deref(), Some("CNN"));

let output = write(&playlist);
assert!(output.starts_with("#EXTM3U"));
```

## Parse Modes

- `parse()` uses `ParseMode::Permissive` to accept common headerless IPTV playlists.
- `parse_strict()` requires `#EXTM3U` as the first non-empty line.
- `parse_with_mode()` lets callers choose explicitly.
- bare URL lines are accepted as URL-only entries

## Main Types

- `M3uPlaylist`
- `M3uHeader`
- `M3uEntry`
- `M3uError`
- `ParseMode`

## Typical Uses

- importing IPTV playlists into applications
- normalizing or cleaning playlists before further processing
- converting raw M3U into shared data models
- round-tripping playlists after edits

## Writer Notes

- all URLs in `entry.urls` are serialized, not just the first one
- `stream_properties`, `vlc_options`, and `web_properties` are written back as directive lines
- `groups` are normalized to a semicolon-delimited `group-title` on output
- identified metadata-only entries are written back even when no URL is present
- serialized `HashMap` metadata is sorted by key for deterministic output
- unknown header and entry extras keep their original key casing across parse/write roundtrips

## Related Crates

- `crispy-iptv-types` for shared cross-protocol domain types
- `crispy-iptv-tools` for deduplication, normalization, and filtering after parsing

## Current Limitations

- the crate does not fetch playlists over the network
- malformed vendor-specific extensions may still need caller-side handling
- writing aims to preserve structured meaning, not exact byte-for-byte source fidelity
- unknown vendor attributes are preserved in `extras`, but unknown directives are skipped

## License

See `LICENSE.md` and `NOTICE.md`.
