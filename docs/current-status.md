# Current Project Status

## Active direction

QuicVid is now a Quinn-based video-call prototype. New product work should use Quinn. Older quiche work is kept as legacy/background material.

## Existing prototypes

### quinn-ping

Purpose:
- basic Quinn client/server experiment
- stream-based ping/pong
- endpoint rebinding/migration sanity check

Status:
- Build: unverified / verified on YYYY-MM-DD
- Run: unverified / verified on YYYY-MM-DD

Commands:

```bash
cd quinn-ping
cargo run --bin server
cargo run --bin client
```

Notes:

* Update this section after verification.

### mouse-coordinates

Purpose:

* Quinn datagram experiment
* sends real-time mouse coordinate telemetry
* closest stepping stone toward fake video frame datagrams

Status:

* Build: unverified / verified on YYYY-MM-DD
* Run: unverified / verified on YYYY-MM-DD

Commands:

```bash
cd mouse-coordinates
cargo run --bin server
cargo run --bin client
```

Notes:

* Update this section after verification.

## Legacy/background work

### quiche experiments

The C/quiche experiments and quiche notes are legacy/background material. They helped explore QUIC migration, path validation, and connection IDs, but new QuicVid product work should use Quinn.

### baseline-study

The baseline-study notes are useful project motivation, but the revised evaluation plan should use a smaller controlled baseline comparison.

## Local setup

Required tools:

```bash
rustup
cargo
```

Recommended checks:

```bash
cd quinn-ping
cargo check

cd ../mouse-coordinates
cargo check
```

## Next implementation step

After Epic 1.1, the next step is Epic 1.2: create the main `quic-vid` app crate with server/client modes, Quinn setup, session IDs, control-stream hello, and JSONL logs.
