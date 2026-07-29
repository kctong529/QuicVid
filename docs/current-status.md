# Current status

Updated after completion of Epic 5.3.

## Working implementation

The active `quic-vid` application supports Quinn client/server modes, QUIC
stream control, generated JPEG video over fragmented QUIC DATAGRAMs,
receiver-side reassembly and preview, transport-independent media runs,
controlled and automatic migration, proactive reconnect, continuous frame IDs
across reconnect, and global completion through the active session.

Migration preserves the media run, session, and Quinn connection. Reconnect
preserves the media run and frame timeline while creating a new connection,
session, and HELLO.

## Measurement pipeline

```text
migration_demo.py
    -> client.log + server.log
    -> recovery_result.py
    -> result.json
    -> recovery_summary.py / recovery_experiment.py
    -> summary.csv
```

Modules:

- `recovery_analysis.py` parses structured events;
- `recovery_identity.py` extracts run/session/connection identities;
- `recovery_frames.py` unions validated frames across sessions;
- `recovery_timing.py` extracts strategy-specific action timing;
- `recovery_continuity.py` measures frame-ID and receiver-time gaps;
- `recovery_result.py` writes versioned per-run JSON;
- `recovery_summary.py` exports flat CSV rows;
- `recovery_experiment.py` runs repeated interleaved trials;
- `verify_recovery.py` checks lifecycle evidence.

## Committed experiment

`results/recovery-experiment-01/` contains 10 migration and 10 reconnect runs.

| Metric | Migration | Reconnect |
|---|---:|---:|
| Success rate | 10/10 | 10/10 |
| Median largest receive gap | 937.4 ms | 802.0 ms |
| Mean largest receive gap | 973.3 ms | 830.2 ms |
| Median missing frames | 2 | 7 |
| Mean missing frames | 2.4 | 7.6 |
| Duplicate frames | 0 | 0 |
| Out-of-order frames | 0 | 0 |

The first experiment shows a trade-off: reconnect resumed receiver activity
sooner, while migration preserved more frames and transport identity.

## Tests

The Python recovery suite contains 63 tests.

```bash
python3 -m unittest discover -s tests -v
python3 -m py_compile scripts/mininet/*.py tests/*.py
```

Rust validation:

```bash
cargo fmt --manifest-path quic-vid/Cargo.toml --check
cargo test --manifest-path quic-vid/Cargo.toml
cargo clippy --manifest-path quic-vid/Cargo.toml   --all-targets --all-features -- -D warnings
cargo build --release --manifest-path quic-vid/Cargo.toml
```

## Active work

Epic 5.4 remains: final aggregate statistics and plots, interpretation and
limitations, verified demo clips, advisor walkthrough, final report, and
submission packaging. No new recovery mechanism is planned.
