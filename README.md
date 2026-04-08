# crispy-m3u

High-performance M3U and M3U8 playlist parser/writer for IPTV workflows.

## Status

Extracted from CrispyTivi. Intended as a reusable Rust parser crate for IPTV playlist ingestion and emission.

## What This Crate Provides

- parse `#EXTM3U` playlists into structured Rust data
- preserve common IPTV metadata such as:
  - `tvg-id`
  - `tvg-name`
  - `tvg-logo`
  - `group-title`
  - catchup metadata
  - custom extras
- write structured playlists back to M3U format
- generate stable identifiers for entries

## Installation

```toml
[dependencies]
crispy-m3u = "0.1"
```

## Quick Start

```rust
use crispy_m3u::{parse, write};

let input = "#EXTM3U\n#EXTINF:-1 tvg-id=\"cnn\" group-title=\"News\",CNN\nhttp://example.com/live/cnn.m3u8\n";
let playlist = parse(input).unwrap();
assert_eq!(playlist.entries.len(), 1);

let output = write(&playlist);
assert!(output.starts_with("#EXTM3U"));
```

## Primary Use Cases

- IPTV app ingestion
- playlist cleaning pipelines
- playlist transformation tools
- migration between IPTV systems

## Relationship To Other Crates

- uses `crispy-iptv-types` for shared domain vocabulary
- pairs well with `crispy-iptv-tools` for normalization and deduplication

## Non-Goals

- network fetching
- Xtream or Stalker API access
- playback probing
- app-specific persistence

## Caveats

- IPTV playlists in the wild are often malformed; parser behavior should be documented with known compatibility notes before public release
- exact write fidelity for vendor-specific extras should be validated in examples/tests
