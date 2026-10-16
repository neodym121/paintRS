# Third-party components

PaintRS itself is MIT-licensed (see `LICENSE`). It embeds or links the following components.

## Embedded assets

| Component | Use | License |
|---|---|---|
| [IBM Plex Sans](https://github.com/IBM/plex) (Regular, Medium, SemiBold; subset to Latin + Cyrillic) | interface font | SIL Open Font License 1.1 — `crates/paint-ui/assets/fonts/LICENSE-IBMPlexSans-OFL.txt` |
| [Lucide](https://lucide.dev) icon font (subset to the icons listed in `tools/icons/icons.txt`) | interface icons | ISC (parts MIT, from Feather) — `crates/paint-ui/assets/fonts/LICENSE-lucide.txt` |

The subsets are produced by `tools/fonts/subset_plex.py` and `tools/icons/build_icons.py` (fontTools).

## Rust crates (main ones)

| Crate | License |
|---|---|
| egui, eframe, egui-wgpu, epaint, egui-winit | MIT OR Apache-2.0 |
| wgpu | MIT OR Apache-2.0 |
| winit | Apache-2.0 |
| image, png, and their codecs | MIT OR Apache-2.0 (codecs: MIT/Apache-2.0/BSD) |
| rfd | MIT |
| serde, toml | MIT OR Apache-2.0 |
| directories | MIT OR Apache-2.0 |
| tracing, tracing-subscriber | MIT |
| thiserror, anyhow, bytemuck | MIT OR Apache-2.0 |

A complete list with versions can be printed with `cargo tree -e normal`.

## Clean-room note

No code, images, icons, fonts, sounds or strings of Microsoft Paint are used. The behavior was
studied only by observing the running program (see `docs/behavior/README.md`).
