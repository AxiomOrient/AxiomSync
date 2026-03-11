# axiomnexus-mobile-ffi

Minimal mobile FFI boundary for `axiomnexus-core`.

## Scope

- Export a stable C ABI for iOS/Swift integration.
- Keep data contracts explicit (`code + owned bytes payload`).
- Keep side effects isolated behind runtime handle methods.
- Be consumed by external mobile app projects (for example, a companion mobile app repository).

## Exported Surface

- `axiomnexus_runtime_new(root_dir, out_runtime)`
- `axiomnexus_runtime_initialize(runtime)`
- `axiomnexus_runtime_backend_status_json(runtime)`
- `axiomnexus_runtime_free(runtime)`
- `axiomnexus_owned_bytes_free(bytes)`

## Data Contract

- Every call returns `AxiomNexusFfiResult`:
  - `code`: `ok | invalid_argument | runtime_error`
  - `payload`: UTF-8 JSON bytes (success payload or structured error payload)
- Returned byte buffers are Rust-owned and must be released with
  `axiomnexus_owned_bytes_free`.

Header and Swift sample:
- C header: `include/axiomnexus_mobile_ffi.h`
- Swift wrapper sample: `examples/swift/AxiomNexusMobileFFI.swift`
  - Note: Swift imports runtime handle type as `OpaquePointer`.

## Build

```bash
cargo build -p axiomnexus-mobile-ffi
```

This crate links `axiomnexus-core` with `default-features = false`, so host-tool execution
(`host-tools` feature) is excluded from mobile builds.

`crate-type = ["staticlib", "cdylib"]` is enabled for native mobile linking.
