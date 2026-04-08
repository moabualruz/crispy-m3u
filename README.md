# crispy-m3u

High-performance M3U and M3U8 playlist parser/writer for IPTV workflows.

## What This Crate Is

`crispy-m3u` parses `#EXTM3U` playlists into structured Rust types and writes them back out. It is intended for IPTV applications, playlist import pipelines, migration tools, and cleanup utilities.

It focuses on the metadata commonly seen in real IPTV playlists, not just the minimal M3U format.

## What It Provides

- `parse(&str) -> Result<M3uPlaylist, M3uError>`
- `write(&M3uPlaylist) -> String`
- stable ID generation for entries
- support for common IPTV metadata such as:
  - `tvg-id`
  - `tvg-name`
  - `tvg-logo`
  - `group-title`
  - catchup fields
  - provider-specific extra attributes

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

## Main Types

- `M3uPlaylist`
- `M3uHeader`
- `M3uEntry`
- `M3uError`

## Typical Uses

- importing IPTV playlists into applications
- normalizing or cleaning playlists before further processing
- converting raw M3U into shared data models
- round-tripping playlists after edits

## Related Crates

- `crispy-iptv-types` for shared cross-protocol domain types
- `crispy-iptv-tools` for deduplication, normalization, and filtering after parsing

## Current Limitations

- the crate does not fetch playlists over the network
- malformed vendor-specific extensions may still need caller-side handling
- writing aims to preserve structured meaning, not exact byte-for-byte source fidelity

## License

See `LICENSE.md` and `NOTICE.md`.
