# Current status

Updated during Epic 5.4 after completing aggregate analysis and result plots.

## Working implementation

The active `quic-vid` application supports Quinn client/server modes, QUIC
stream control, generated JPEG video over fragmented QUIC DATAGRAMs,
receiver-side reassembly and preview, transport-independent media runs,
controlled and automatic migration, proactive reconnect, continuous frame IDs
across reconnect, and global completion through the active session.

Migration preserves the media run, session, and Quinn connection. Reconnect
preserves the media run and frame timeline while creating a new connection,
session, and HELLO.

## Measurement and comparison pipeline

```text
migration_demo.py
    -> client.log + server.log
    -> recovery_result.py
    -> result.json
    -> recovery_summary.py
    -> trial summary.csv
    -> recovery_compare.py
    -> aggregate summary.csv + summary.md
    -> recovery_plot_alternatives.py
    -> final figures
```

## Committed experiment

`results/recovery-experiment-01/` contains ten migration and ten reconnect
runs. All 20 completed successfully and produced zero analysis errors.

| Metric | Migration | Reconnect |
|---|---:|---:|
| Success rate | 10/10 | 10/10 |
| Median largest receive gap | 937.4 ms | 802.0 ms |
| Mean largest receive gap | 973.3 ms | 830.2 ms |
| Median missing frames | 2 | 7 |
| Mean missing frames | 2.4 | 7.6 |
| Duplicate frames | 0 | 0 |
| Out-of-order frames | 0 | 0 |

The comparison shows a measured trade-off:

- reconnect resumed receiver activity sooner;
- migration preserved more frames;
- migration retained one Quinn connection and QuicVid session;
- reconnect created a replacement connection and session;
- both preserved the same logical media run and global frame timeline.

Final result figures:

```text
results/recovery-experiment-01/analysis/plots/
├── receive-gap-histogram.png
└── missing-frames-frequency.png
```

The migration outlier `migrate-003` is retained in all statistics and plots.

## Metric interpretation

`largest_receive_gap_ms` is the primary cross-strategy interruption metric.

Strategy-specific action duration is diagnostic only because migration and
reconnect use different completion events.

Reconnect frame loss is aggregated across both transport sessions by
`media_run_id`; the S2-local receive summary is not treated as the global
result.

## Tests

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

## Remaining Epic 5.4 work

- produce one verified final migration run;
- produce one verified final reconnect run;
- record short demonstration clips;
- prepare the advisor-facing walkthrough;
- update the final report;
- validate the documented workflow from a clean checkout;
- prepare the submission package.

No new recovery mechanism is planned.
