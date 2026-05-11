# Current Project Status

## Active direction

QuicVid is now a Quinn-based video-call prototype. New product work should use Quinn. Older quiche work is kept as legacy/background material.

## Existing prototypes

### quinn-ping

Purpose:
- Basic Quinn client/server prototype.
- Useful reference for Epic 1.2, especially Quinn endpoint setup, stream use, and connection/rebinding behavior.

Cargo package:
- `quinn-test`

Binaries:
- `client`
- `server`

Build status:
- Verified on: 2026-05-11
- Commands:

```bash
cd quinn-ping
cargo metadata --no-deps --format-version 1
cargo check
cargo build
```

Result:

* `cargo check` succeeds.
* `cargo build` succeeds.
* Warning: `src/bin/server.rs` has an unused import, `net::SocketAddr`.

Runtime status:

* Pending runtime verification.

Runtime notes from source inspection:

* server listens on `0.0.0.0:4433`
* client binds locally to `0.0.0.0:0`
* client currently targets `10.0.0.1:4433`
* client sends `PING` messages and expects `PONG`
* client triggers endpoint rebinding every 5 rounds with `endpoint.rebind(...)`
* the hard-coded `10.0.0.1:4433` target appears Mininet-oriented, so local two-process verification may require changing the target address to `127.0.0.1:4433` or making it configurable

Notes:

* Keep as a reference for building the main `quic-vid` client/server skeleton in Epic 1.2.

### mouse-coordinates

Purpose:
- Quinn datagram prototype.
- Useful reference for Epic 1.3, especially fake-video datagram streaming.

Cargo package:
- `mouse-coordinates`

Binaries:
- `client`
- `server`

Build status:
- Verified on: 2026-05-11
- Commands:

```bash
cd mouse-coordinates
cargo metadata --no-deps --format-version 1
cargo check
cargo build
````

Result:

* `cargo check` succeeds.
* `cargo build` succeeds.

Runtime status:

* Pending runtime verification.

Notes:

* Expected to inform the fake-video frame datagram path in Epic 1.3.

## Legacy/background work

### quiche experiments

The C/quiche experiments and quiche notes are legacy/background material. They helped explore QUIC migration, path validation, and connection IDs, but new QuicVid product work should use Quinn.

### baseline-study

The baseline-study notes are useful project motivation, but the revised evaluation plan should use a smaller controlled baseline comparison.

## Local setup notes

Useful prototype checks:

```bash
cd quinn-ping
cargo check
cargo build

cd ../mouse-coordinates
cargo check
cargo build
```

## Next implementation step

Epic 1.2: create the main `quic-vid` app skeleton with server/client modes, Quinn setup, session IDs, an initial control-stream hello, and structured JSONL logs.
