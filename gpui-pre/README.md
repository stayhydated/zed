# GPUI capture packages

The `linux`, `platform`, and `wgpu` packages compile the canonical Zed sources
for applications using `gpui-pre =0.3.11`. The capture branch includes upstream
`main` at `784374c679` and retains Linux window-image readback through the
window's renderer and glyph atlas. Its Wgpu dependency follows the exact
revision selected by that upstream source. The canonical Linux and WGPU runtime
sources
are unchanged since the published snapshot's recorded Zed revision. Package
adapters retain their selected Wgpu APIs; GPUI core and benchmark changes follow
their own upstream release boundary.

`Cargo.lock` records the standalone package graph. Validate the backend with
these focused commands:

```sh
cargo check --manifest-path gpui-pre/Cargo.toml \
  -p gpui-pre-platform --features test-support --locked
cargo test --manifest-path gpui-pre/Cargo.toml \
  -p gpui-pre-wgpu --lib --features test-support --locked
```

The `wayland_capture` integration test requires a private Wayland compositor;
run its ignored test with `--features test-support -- --ignored`.

Applications maintaining a published-backend snapshot can replay the bounded
capture difference against the matching published GPUI sources. Keep their
snapshot and patch provenance beside those sources.
