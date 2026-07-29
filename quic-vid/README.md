# QuicVid application

`quic-vid` is the active Quinn-based media prototype.

## Features

- client and server modes;
- QUIC stream control;
- generated JPEG test-pattern frames;
- QUIC DATAGRAM fragmentation and reassembly;
- optional receiver preview;
- transport-independent media runs;
- controlled and automatic migration;
- proactive reconnect;
- structured recovery and frame-validation events.

## Identity model

```text
media_run_id  logical call/experiment, preserved by both strategies
session_id    transport session, preserved only by migration
frame_id      global media-timeline position, never restarts on reconnect
```

## Run locally

```bash
cargo run --manifest-path quic-vid/Cargo.toml --   server --listen 127.0.0.1:4433
```

```bash
cargo run --manifest-path quic-vid/Cargo.toml --   client   --connect 127.0.0.1:4433   --bind 127.0.0.1:0   --fps 10   --duration-seconds 6
```

Use `--help` for all current options.

JPEG frames may be split across several QUIC DATAGRAMs. The receiver reassembles
and validates complete frames. Control streams associate each transport
session with one media run.

Migration keeps the existing connection/session. Reconnect creates another
connection/session and HELLO for the same media run, then resumes the current
frame timeline.

Automatic recovery shares:

```text
Healthy -> Suspect -> Challenging
```

Migration calls `Endpoint::rebind()` and waits for resumed ACK progress.
Reconnect creates a replacement endpoint/connection/session.

See `docs/mininet-migration.md` and `docs/recovery-analysis.md`.

## Validation

```bash
cargo fmt --manifest-path quic-vid/Cargo.toml --check
cargo test --manifest-path quic-vid/Cargo.toml
cargo clippy --manifest-path quic-vid/Cargo.toml   --all-targets --all-features -- -D warnings
```
