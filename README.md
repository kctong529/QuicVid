# QuicVid

QuicVid is a Quinn-based Rust prototype for evaluating media continuity during
network-path failure.

It compares two proactive recovery strategies under the same Mininet failure
scenario:

- **QUIC migration** rebinds the existing Quinn endpoint and preserves the
  connection and QuicVid session.
- **QUIC reconnect** creates a replacement connection and session while
  preserving the same logical media run and global frame timeline.

Generated JPEG test-pattern frames travel over QUIC DATAGRAMs. QUIC streams
carry session control messages, and structured logs feed a repeatable Python
analysis pipeline.

## Project background

QuicVid began in September with a broader plan to build a QUIC-based video-call
system using quiche, Qt, FFmpeg, mobile-style handover experiments,
commercial-application comparisons, and possibly user-study-style evaluation.

That plan established the original motivation, but it did not become the final
implementation path. The early phase instead produced smaller experiments for
understanding QUIC connection migration, especially the Quinn-based
`quinn-ping` and `mouse-coordinates` prototypes. Those experiments established
connection setup, QUIC DATAGRAM traffic, endpoint rebinding, and connection
continuity across address changes.

The current project builds on that foundation with a focused Quinn-based media
prototype, a deterministic dual-path Mininet environment, proactive migration
and reconnect strategies, and a reproducible measurement pipeline.

## Academic context

- **Student:** Tong Ki Chun
- **Advisor:** Pasi Sarolahti
- **Institution:** Aalto University, School of Electrical Engineering
- **Project type:** Bachelor's final project

## Engineering problem and project goal

Video calls are long-lived interactive sessions, but a network path may change
while the user still considers the call to be the same logical session. A
wireless interruption, address change, path failure, or changed UDP binding can
force a conventional application to reconnect and recreate transport or
application state. To the user, this may appear as frozen media, frame loss, a
long recovery pause, or a dropped call.

QUIC provides a different design point. A connection is identified by
connection IDs rather than only by its current IP address and port, so an
endpoint can validate and adopt another path while retaining the same logical
transport connection.

QuicVid asks the following engineering question:

> Can a media prototype use QUIC connection migration to preserve transport and
> application-session continuity across a controlled path failure, and how does
> that behavior compare with proactive reconnect using the same failure
> detector?

The project goal is a controlled proof of feasibility:

1. send a repeatable media workload between separate Quinn client and server
   processes;
2. trigger a sustained failure of the active path during the media run;
3. recover either by migrating the existing connection or proactively
   reconnecting on the alternate path;
4. preserve one logical `MediaRun` and continuous frame timeline;
5. measure receiver-visible interruption, global frame loss, completion, and
   transport identity;
6. produce reproducible experiment records suitable for the final report.

The goal is not to build a production Zoom replacement. The current prototype
does not attempt public-Internet NAT traversal, conferencing, adaptive media,
production retry behavior, or physical mobile-network handover.

## Important note about the old plan

Earlier planning documents are project history, not the current implementation
contract.

The active implementation uses Rust and Quinn, generated JPEG test-pattern
frames, QUIC DATAGRAM media transport, QUIC stream control messages, a
dual-path Mininet topology, and Python-based experiment analysis. The earlier
quiche, Qt, FFmpeg, broad commercial-app comparison, and user-study ideas are
not required deliverables.

The project should therefore be evaluated against the implemented engineering
goal: demonstrating and measuring media continuity during controlled path
failure, including a fair migration-versus-reconnect comparison—not against the
original aspirational stack or scope.

## Current result

The first committed experiment contains 10 migration and 10 reconnect trials.
All 20 runs completed successfully.

| Metric | Migration | Reconnect |
|---|---:|---:|
| Successful runs | 10/10 | 10/10 |
| Median largest receive gap | 937.4 ms | 802.0 ms |
| Mean largest receive gap | 973.3 ms | 830.2 ms |
| Median missing frames | 2 | 7 |
| Mean missing frames | 2.4 | 7.6 |
| Median recovery-action duration | 156 ms | 1 ms |

Reconnect restored receiver activity sooner in this experiment, while
migration preserved more frames and retained one connection and session.
Strategy-specific action duration is diagnostic; receiver-side frame gap is the
primary cross-strategy interruption metric.

The committed dataset is under
[`results/recovery-experiment-01/`](results/recovery-experiment-01/).

## Repository map

```text
quic-vid/                  active Quinn media prototype
scripts/mininet/           topology, launchers, verification, and analysis
tests/                     Python recovery-analysis tests
results/                   compact committed experiment datasets
docs/current-status.md     implementation and milestone status
docs/mininet-migration.md  runnable Mininet workflows
docs/recovery-analysis.md  result schema and analysis pipeline
PLAN.md                    current scope and roadmap

quinn-ping/                earlier Quinn migration experiment
mouse-coordinates/         earlier Quinn DATAGRAM experiment
docs/quiche-*.md           legacy quiche study notes
```

New implementation work belongs in `quic-vid/` and `scripts/mininet/`. Earlier
Quinn and quiche material is retained as project history.

## Build

```bash
cargo build --release --manifest-path quic-vid/Cargo.toml
sudo mn -c
```

The launcher expects `quic-vid/target/release/quic-vid`.

## Run one recovery demo

Interactive migration:

```bash
sudo python3 scripts/mininet/migration_demo.py   --preset health-sustained   --log-dir /tmp/quicvid-migrate
```

Interactive reconnect:

```bash
sudo python3 scripts/mininet/migration_demo.py   --preset reconnect-sustained   --log-dir /tmp/quicvid-reconnect
```

Noninteractive migration:

```bash
sudo python3 scripts/mininet/migration_demo.py   --noninteractive   --preset health-sustained   --recovery-strategy migrate   --no-quiet-media-logs   --log-dir /tmp/quicvid-migrate
```

Noninteractive reconnect:

```bash
sudo python3 scripts/mininet/migration_demo.py   --noninteractive   --preset health-sustained   --recovery-strategy reconnect   --no-quiet-media-logs   --log-dir /tmp/quicvid-reconnect
```

## Verify and analyze one run

```bash
python3 scripts/mininet/verify_recovery.py   --strategy migrate   --client-log /tmp/quicvid-migrate/client.log   --server-log /tmp/quicvid-migrate/server.log
```

```bash
python3 -m scripts.mininet.recovery_result   --client-log /tmp/quicvid-migrate/client.log   --server-log /tmp/quicvid-migrate/server.log   --output /tmp/quicvid-migrate/result.json
```

## Run repeated experiments

```bash
sudo python3 -m scripts.mininet.recovery_experiment   --output-root artifacts/recovery-experiment   --repetitions 10   --scenario-command   '["python3","scripts/mininet/migration_demo.py","--noninteractive","--preset","health-sustained","--recovery-strategy","{strategy}","--no-quiet-media-logs","--log-dir","{trial_dir}"]'
```

The runner interleaves strategies, writes one `result.json` per completed run,
and generates `summary.csv`.

## Documentation

- [`PLAN.md`](PLAN.md)
- [`docs/current-status.md`](docs/current-status.md)
- [`docs/mininet-migration.md`](docs/mininet-migration.md)
- [`docs/recovery-analysis.md`](docs/recovery-analysis.md)
- [`quic-vid/README.md`](quic-vid/README.md)
- [`results/recovery-experiment-01/README.md`](results/recovery-experiment-01/README.md)

## Validation

```bash
cargo fmt --manifest-path quic-vid/Cargo.toml --check
cargo test --manifest-path quic-vid/Cargo.toml
cargo clippy --manifest-path quic-vid/Cargo.toml   --all-targets --all-features -- -D warnings
python3 -m unittest discover -s tests -v
python3 -m py_compile scripts/mininet/*.py tests/*.py
```

## Scope

QuicVid is a controlled proof of concept, not a production video-call system.
The current evaluation uses a deterministic Mininet topology, one sustained
failure pattern, one media configuration, and one detector configuration. It
does not include physical Wi-Fi handover, NetworkManager-triggered recovery,
NAT traversal, conferencing, adaptive media, a TCP baseline, or
timeout-triggered reconnect.
