# Mininet recovery workflows

See [`recovery-analysis.md`](recovery-analysis.md) for result extraction.

## Topology

```text
                    Path A
client-eth0 ---- r1 --------\
                             server
client-eth1 ---- r2 --------/
                    Path B
```

The client initially uses `10.0.1.2` on Path A. Recovery selects `10.0.2.2` on
Path B.

## Prerequisites

```bash
cargo build --release --manifest-path quic-vid/Cargo.toml
sudo mn -c
```

## Presets

| Preset | Purpose |
|---|---|
| `diagnostic` | short controlled rebind |
| `preview` | visual controlled migration |
| `health-transient` | temporary outage that recovers before challenge |
| `health-sustained` | sustained failure with migration |
| `reconnect-sustained` | sustained failure with reconnect |

## Interactive demos

```bash
sudo python3 scripts/mininet/migration_demo.py   --preset health-sustained   --log-dir /tmp/quicvid-migrate
```

```bash
sudo python3 scripts/mininet/migration_demo.py   --preset reconnect-sustained   --log-dir /tmp/quicvid-reconnect
```

## Noninteractive demos

Noninteractive mode starts all processes directly, starts the client
immediately, waits for completion, drains late output, cleans up, and stops
Mininet. `--log-dir` is required.

```bash
sudo python3 scripts/mininet/migration_demo.py   --noninteractive   --preset health-sustained   --recovery-strategy migrate   --no-quiet-media-logs   --log-dir /tmp/quicvid-migrate
```

```bash
sudo python3 scripts/mininet/migration_demo.py   --noninteractive   --preset health-sustained   --recovery-strategy reconnect   --no-quiet-media-logs   --log-dir /tmp/quicvid-reconnect
```

Using one preset and changing only `--recovery-strategy` keeps experiment
parameters equal.

## Expected identities

Migration:

```text
one media run
one session
one Quinn connection
one HELLO
Path A -> Path B
```

Reconnect:

```text
one media run
two sessions
two Quinn connections
two HELLOs
Path A -> Path B
no frame-timeline restart
```

The old S1 server task may remain alive until Quinn's idle timeout because a
close sent over failed Path A may not reach the server.

## Verify a run

```bash
python3 scripts/mininet/verify_recovery.py   --strategy migrate   --client-log /tmp/quicvid-migrate/client.log   --server-log /tmp/quicvid-migrate/server.log
```

```bash
python3 scripts/mininet/verify_recovery.py   --strategy reconnect   --client-log /tmp/quicvid-reconnect/client.log   --server-log /tmp/quicvid-reconnect/server.log
```

## Analyze a run

```bash
python3 -m scripts.mininet.recovery_result   --client-log /tmp/quicvid-reconnect/client.log   --server-log /tmp/quicvid-reconnect/server.log   --output /tmp/quicvid-reconnect/result.json
```

## Export several results

```bash
python3 -m scripts.mininet.recovery_summary   /tmp/quicvid-migrate/result.json   /tmp/quicvid-reconnect/result.json   --output /tmp/quicvid-summary.csv
```

## Run repeated experiments

```bash
sudo python3 -m scripts.mininet.recovery_experiment   --output-root artifacts/recovery-experiment   --repetitions 10   --scenario-command   '["python3","scripts/mininet/migration_demo.py","--noninteractive","--preset","health-sustained","--recovery-strategy","{strategy}","--no-quiet-media-logs","--log-dir","{trial_dir}"]'
```

Trials are interleaved as `migrate-001`, `reconnect-001`, and so on. Each
completed trial contains logs, command metadata, runner output, and
`result.json`; the experiment root contains `summary.csv`.

Compact evidence is committed under `results/recovery-experiment-01/`. Raw logs
and pcaps are excluded because they are bulky and reproducible.

## Troubleshooting

```bash
sudo mn -c
sudo mn --switch ovsbr --test pingall
test -x quic-vid/target/release/quic-vid
python3 scripts/mininet/migration_demo.py --help
python3 -m scripts.mininet.recovery_experiment --help
```
