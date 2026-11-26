# GPUI capture packages

These three Cargo patch packages provide the Linux Wayland and X11 window
capture implementation for the GPUI Pre 0.3.14 stack used by GPUI Kit 0.11.
Their canonical Zed sources are based on
`f126950f9d3b62666f97437ae5d4ccac2272c4db`, the exact Zed revision recorded
in the published packages. The adapters keep the registry GPUI package names
and versions so consumers can keep a single GPUI type graph. They use released
wgpu 30.0.1, matching the published GPUI Pre renderer baseline; the upstream
Zed workspace keeps its own wgpu git pin.

Capture under `test-support` reuses the live window renderer and its atlas.
It draws into an offscreen texture and reads it back without a surface acquire
or presentation. The Wayland geometry update remains ordered after the
drawable resize.

Validate the standalone packages using Rust 1.100:

```sh
cargo +1.100.0 check --manifest-path gpui-pre/Cargo.toml \
  -p gpui-pre-platform --features wayland,test-support --locked
cargo +1.100.0 test --manifest-path gpui-pre/Cargo.toml \
  -p gpui-pre-wgpu --lib --features test-support --locked
```

With a running private Wayland compositor and a usable GPU/software adapter,
run the ignored window test:

```sh
cargo +1.100.0 test --manifest-path gpui-pre/Cargo.toml \
  -p gpui-pre-linux --features wayland,test-support --locked \
  --test wayland_capture -- --ignored
```
