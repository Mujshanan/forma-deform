# Vendored AniCraft plug-in SDK

This crate is a vendored copy of the guest-side Rust SDK from
[Mujshanan/anicraft](https://github.com/Mujshanan/anicraft).

- Upstream source: `examples/plugins/sdk/src/lib.rs`
- Upstream commit: [bcfea816478f2a00ab172cefe67f0bd5696b3886](https://github.com/Mujshanan/anicraft/commit/bcfea816478f2a00ab172cefe67f0bd5696b3886)
- Upstream package license: `MIT OR Apache-2.0`
- ABI: v1

The upstream package inherits its version, edition, and license from AniCraft's
sample-plugin workspace. This standalone manifest makes the SDK usable from
Forma Deform without adding the entire AniCraft repository as a submodule or
depending on an unpublished path-only package.

## Update procedure

When updating AniCraft integration, compare this copy with the upstream SDK at
the pinned source path, review ABI changes, update this note to the new upstream
commit, and run the native workspace tests plus the `wasm32-unknown-unknown`
build. Do not silently track upstream `main`.
