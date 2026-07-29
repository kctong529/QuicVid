# QuicVid

QuicVid is a Quinn-based Rust prototype for evaluating media continuity during
a controlled network-path failure.

It compares two proactive recovery strategies under the same dual-path Mininet
scenario:

- **QUIC migration** rebinds the existing Quinn endpoint and preserves the
  connection and QuicVid session.
- **Proactive reconnect** creates a replacement connection and session while
  preserving the same logical media run and global frame timeline.

Generated JPEG test-pattern frames travel over QUIC DATAGRAMs. QUIC streams
carry session-control messages, and structured logs feed a reproducible Python
analysis pipeline.

## Quick evaluation path

For a concise overview of the completed work:

1. Read the [engineering question and scope](#engineering-question-and-scope).
2. Review the [measured result](#measured-result).
3. Open the
   [full recovery comparison](results/recovery-experiment-01/analysis/summary.md),
   including the final figures and limitations.
4. Watch the
   [preview-mode recovery demonstration](results/final-demo/simultaneously-preview.mp4).
5. Inspect the active implementation under [`quic-vid/`](quic-vid/) and the
   experiment tooling under [`scripts/mininet/`](scripts/mininet/).
6. Run the [validation commands](#validation).

The compact committed experiment evidence is under
[`results/recovery-experiment-01/`](results/recovery-experiment-01/).

## Academic context

- **Student:** Tong Ki Chun
- **Advisor:** Pasi Sarolahti
- **Institution:** Aalto University, School of Electrical Engineering
- **Project type:** Bachelor's final project

## Engineering question and scope

Video calls are long-lived interactive sessions, but their active network path
may fail or change while the user still considers the call to be the same
logical session. A conventional reconnect can restore communication, but it
replaces transport and application-session state.

QuicVid asks:

> Can a media prototype use QUIC connection migration to preserve transport and
> application-session continuity across a controlled path failure, and how does
> that behavior compare with proactive reconnect using the same failure
> detector?

The project demonstrates this through:

1. a repeatable generated-JPEG media workload;
2. separate Quinn client and server processes;
3. one sustained Path A failure during each media run;
4. recovery through either migration or proactive reconnect;
5. one transport-independent `MediaRun` and continuous frame timeline;
6. receiver-side interruption, frame-loss, completion, and identity metrics;
7. repeated, automated, and reproducible Mininet experiments.

This is a controlled proof of concept rather than a production video-call
system. Physical Wi-Fi handover, NAT traversal, conferencing, adaptive media,
production retry behavior, TCP comparison, and user studies are outside the
final scope.

## Measured result

The committed experiment contains 10 migration and 10 reconnect trials. All 20
runs completed successfully and produced no analysis errors.

| Metric | Migration | Reconnect |
|---|---:|---:|
| Successful runs | 10/10 | 10/10 |
| Median largest receiver-observed gap | 937.4 ms | 802.0 ms |
| Mean largest receiver-observed gap | 973.3 ms | 830.2 ms |
| Median missing frames | 2 | 7 |
| Mean missing frames | 2.4 | 7.6 |
| Quinn connections | 1 | 2 |
| QuicVid sessions | 1 | 2 |

The result shows a trade-off:

- reconnect resumed receiver activity sooner;
- migration preserved more media frames;
- migration retained the existing Quinn connection and QuicVid session;
- reconnect created a replacement connection and session;
- both strategies preserved the same logical media run and frame timeline.

`largest_receive_gap_ms` is the primary cross-strategy interruption metric.
Strategy-specific action duration is diagnostic because migration and reconnect
use different completion events.

Full statistics, figures, interpretation, and limitations:

- [Recovery comparison](results/recovery-experiment-01/analysis/summary.md)
- [Experiment dataset overview](results/recovery-experiment-01/README.md)

## Final figures

![Distribution of receiver-visible interruption](results/recovery-experiment-01/analysis/plots/receive-gap-histogram.png)

*Distribution of the largest receiver-observed frame gap across ten trials per
strategy. Reconnect produced shorter interruptions in this controlled setup,
while one migration trial produced a larger outlier.*

![Frequency of global missing-frame counts](results/recovery-experiment-01/analysis/plots/missing-frames-frequency.png)

*Frequency of global missing-frame counts across ten trials per strategy.
Migration most commonly lost two frames, while reconnect most commonly lost
seven or eight.*

## Implications for real-world video chat applications

The experiment does not prove production readiness, but it shows where QUIC
migration could be valuable in a real video-chat architecture.

A video call is usually a logical application session that should survive
changes below it. A user may move between Wi-Fi and cellular connectivity,
experience a local address change, or temporarily lose the active path without
intending to end the call. The migration strategy demonstrated here preserves
the existing QUIC connection and QuicVid session while changing the local
network path. In a real application, preserving those identities could reduce
the amount of state that must be recreated after a path change.

Potential benefits include:

- avoiding a new application-session handshake;
- preserving transport-level state associated with the existing connection;
- keeping call identity, media-run state, and control-channel state attached to
  the same transport connection;
- reducing frame loss during recovery, as observed in this controlled test;
- simplifying recovery logic above the transport layer because the application
  does not need to treat every path change as a new call session.

The measured result also shows that migration should not automatically replace
reconnect in every situation. In this experiment, proactive reconnect resumed
receiver activity sooner, while migration preserved more frames and retained
transport continuity. A production system could therefore use a layered
strategy:

1. attempt migration when an alternate local path is available and preserving
   connection state is valuable;
2. confirm resumed delivery using receiver or ACK progress;
3. fall back to proactive reconnect when migration cannot be confirmed within
   an application-defined deadline;
4. preserve the logical call and media timeline above either transport action.

This suggests that QUIC migration is best understood as one recovery mechanism
inside a broader call-continuity design, not as a complete solution by itself.
A real deployment would still need path validation, NAT and firewall handling,
network-interface monitoring, congestion-control adaptation, media buffering,
security review, fallback behavior, and testing on physical Wi-Fi and cellular
networks.

The current Mininet result is therefore evidence of engineering feasibility:
migration can preserve connection and application-session identity across a
controlled path failure, and it can provide a different loss/interruption
trade-off from reconnect. It does not establish how large the benefit would be
on real networks.

## Recovery behavior

Both strategies use the same proactive health detector:

```text
Healthy -> Suspect -> Challenging
                         |
                         +-- migrate
                         |      preserve connection and session
                         |      rebind the existing Quinn endpoint
                         |
                         +-- reconnect
                                create a replacement connection and session
                                preserve the MediaRun and frame timeline
```

Expected identity:

| Property | Migration | Reconnect |
|---|---:|---:|
| Logical media runs | 1 | 1 |
| Quinn connections | 1 | 2 |
| QuicVid sessions | 1 | 2 |
| HELLO exchanges | 1 | 2 |
| Global frame timeline preserved | Yes | Yes |

## Repository map

```text
quic-vid/                  active Quinn media prototype
scripts/mininet/           topology, launchers, verification, and analysis
tests/                     Python recovery-analysis tests
results/                   committed experiment evidence and final demo
docs/current-status.md     final implementation and validation status
docs/mininet-migration.md  runnable Mininet workflows
docs/recovery-analysis.md  result schema and analysis pipeline
PLAN.md                    project-management history and completion record

quinn-ping/                earlier Quinn migration experiment
mouse-coordinates/         earlier Quinn DATAGRAM experiment
docs/quiche-*.md           legacy quiche study notes
```

New implementation work belongs in `quic-vid/` and `scripts/mininet/`.
The earlier Quinn and quiche material is retained as project history.

## Build

```bash
cargo build --release --manifest-path quic-vid/Cargo.toml
sudo mn -c
```

The Mininet launcher expects:

```text
quic-vid/target/release/quic-vid
```

## Run one recovery demo

Interactive migration:

```bash
sudo python3 scripts/mininet/migration_demo.py \
  --preset health-sustained \
  --log-dir /tmp/quicvid-migrate
```

Interactive reconnect:

```bash
sudo python3 scripts/mininet/migration_demo.py \
  --preset reconnect-sustained \
  --log-dir /tmp/quicvid-reconnect
```

Equivalent noninteractive runs using one shared preset:

```bash
sudo python3 scripts/mininet/migration_demo.py \
  --noninteractive \
  --preset health-sustained \
  --recovery-strategy migrate \
  --no-quiet-media-logs \
  --log-dir /tmp/quicvid-migrate
```

```bash
sudo python3 scripts/mininet/migration_demo.py \
  --noninteractive \
  --preset health-sustained \
  --recovery-strategy reconnect \
  --no-quiet-media-logs \
  --log-dir /tmp/quicvid-reconnect
```

Detailed workflow:

- [Mininet recovery workflows](docs/mininet-migration.md)
- [Recovery analysis and experiment data](docs/recovery-analysis.md)

## Verify and analyze one run

```bash
python3 scripts/mininet/verify_recovery.py \
  --strategy migrate \
  --client-log /tmp/quicvid-migrate/client.log \
  --server-log /tmp/quicvid-migrate/server.log
```

```bash
python3 -m scripts.mininet.recovery_result \
  --client-log /tmp/quicvid-migrate/client.log \
  --server-log /tmp/quicvid-migrate/server.log \
  --output /tmp/quicvid-migrate/result.json
```

## Run the repeated experiment

```bash
sudo python3 -m scripts.mininet.recovery_experiment \
  --output-root artifacts/recovery-experiment \
  --repetitions 10 \
  --scenario-command \
  '["python3","scripts/mininet/migration_demo.py","--noninteractive","--preset","health-sustained","--recovery-strategy","{strategy}","--no-quiet-media-logs","--log-dir","{trial_dir}"]'
```

The runner interleaves migration and reconnect trials, writes one structured
`result.json` per completed run, and generates a flat `summary.csv`.

## Validation

Rust:

```bash
cargo fmt --manifest-path quic-vid/Cargo.toml --check
cargo test --manifest-path quic-vid/Cargo.toml
cargo clippy --manifest-path quic-vid/Cargo.toml \
  --all-targets --all-features -- -D warnings
cargo build --release --manifest-path quic-vid/Cargo.toml
```

Python:

```bash
python3 -m unittest discover -s tests -v
python3 -m py_compile scripts/mininet/*.py tests/*.py
```

## Reproducibility environment

Recorded environment:

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

The version capture did not report a Mininet version. `matplotlib` was not
installed in this runtime, so no matplotlib version is claimed here.

At capture time, `final-environment.txt` and
`scripts/mininet/__pycache__/` were untracked. Remove or ignore them before the
final clean-checkout validation.

## Project background and scope change

QuicVid began with a broader proposal involving quiche, Qt, FFmpeg,
mobile-style handover, commercial-application comparisons, and possible
user-study-style evaluation.

Early feasibility work showed that this scope was too broad for the available
project time. The active project therefore became a focused Quinn-based media
prototype with deterministic Mininet experiments and a reproducible evaluation
pipeline.

Earlier planning documents are project history, not the current implementation
contract. The project should be evaluated against the implemented engineering
goal described above, not against the original aspirational stack.

## Further documentation

- [Project plan and completion record](PLAN.md)
- [Current implementation status](docs/current-status.md)
- [Mininet recovery workflows](docs/mininet-migration.md)
- [Recovery analysis pipeline](docs/recovery-analysis.md)
- [Active Rust prototype](quic-vid/README.md)
- [Committed experiment dataset](results/recovery-experiment-01/README.md)
