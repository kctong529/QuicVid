# Current status

Updated after completing the comparative analysis, final figures,
demonstration clip, and advisor-facing documentation.

## Project state

The implementation and comparative evaluation are complete.

The active `quic-vid` application supports:

- Quinn client and server modes;
- QUIC stream control messages;
- generated JPEG video over fragmented QUIC DATAGRAMs;
- receiver-side reassembly and preview;
- transport-independent media runs;
- controlled and automatic migration;
- proactive reconnect;
- continuous frame IDs across reconnect;
- completion through the currently active session;
- structured logging for automated verification and analysis.

Migration preserves the media run, QuicVid session, and Quinn connection.
Reconnect preserves the media run and frame timeline while creating a
replacement connection, session, and HELLO.

No further recovery mechanism is planned.

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
trials. All 20 completed successfully and produced zero analysis errors.

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
`media_run_id`; the replacement session's local result is not treated as the
media-run-wide result.

## Final demonstration

A preview-mode demonstration is committed at:

```text
results/final-demo/simultaneously-preview.mp4
```

The clip shows the preview continuing through the recovery workflow while the
terminal output remains visible. The structured logs and result JSON remain the
primary validation evidence; the video is supplementary visual evidence.

## Reproducibility environment

Recorded final environment:

```text
Git revision:       631d82d18d5cd4542f3132078a14fb6a7815fda6
Operating system:   Ubuntu 24.04.1 LTS
Kernel:             Linux 6.8.0-136-generic, aarch64
Python:             3.12.3
Pillow:             10.2.0
Rust compiler:      rustc 1.92.0
Cargo:              1.92.0
Open vSwitch:       3.3.4
iproute2:           6.1.0
```

The captured output did not report a Mininet version. `matplotlib` was also not
installed in this runtime, so no matplotlib version should be claimed for the
final validation environment.

At capture time the worktree contained two untracked/generated entries:

```text
final-environment.txt
scripts/mininet/__pycache__/
```

These should be removed or ignored before clean-checkout validation.

## Validation commands

Rust:

```bash
cargo fmt --manifest-path quic-vid/Cargo.toml --check
cargo test --manifest-path quic-vid/Cargo.toml
cargo clippy --manifest-path quic-vid/Cargo.toml   --all-targets --all-features -- -D warnings
cargo build --release --manifest-path quic-vid/Cargo.toml
```

Python:

```bash
python3 -m unittest discover -s tests -v
python3 -m py_compile scripts/mininet/*.py tests/*.py
```

Mininet smoke checks:

```bash
sudo mn -c
sudo mn --switch ovsbr --test pingall
test -x quic-vid/target/release/quic-vid
python3 scripts/mininet/migration_demo.py --help
python3 -m scripts.mininet.recovery_experiment --help
```

## Remaining finalization work

Implementation and experiment work are complete. Remaining tasks are:

- remove temporary files and Python caches;
- run the complete validation suite from a clean worktree or checkout;
- record the validation result;
- finish the final report;
- prepare the advisor walkthrough;
- prepare the submission package.

No new recovery implementation is required.
