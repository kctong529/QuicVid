# Recovery analysis and experiment data

## Pipeline

```text
client.log + server.log
        |
        v
recovery_analysis.py
        |
        +--> recovery_identity.py
        +--> recovery_frames.py
        +--> recovery_timing.py
        +--> recovery_continuity.py
        |
        v
recovery_result.py -> result.json
        |
        +--> recovery_summary.py -> summary.csv
        |
        +--> recovery_experiment.py -> repeated trials
```

The parser accepts `event=<name> key=value ...` lines, including prefixes and
quoted values. UUIDs and addresses remain strings. Malformed structured lines
are reported; ordinary output is ignored.

## Identity and aggregation

One result records strategy, `media_run_id`, frame boundary, session IDs, Quinn
connection IDs, HELLO sessions, addresses, and completion state.

| Property | Migration | Reconnect |
|---|---:|---:|
| Media runs | 1 | 1 |
| Sessions | 1 | 2 |
| Connections | 1 | 2 |
| HELLOs | 1 | 2 |

Reconnect frames are aggregated by media run:

```text
received_frames(run) =
    received_frames(session 1)
    union
    received_frames(session 2)
```

The S2-local summary is not the media-run-wide result.

## Timing and continuity

```text
migration:
    automatic_rebind_started -> migration_confirmed

reconnect:
    reconnect_started -> reconnect_completed
```

These completion events have different semantics.

Receiver continuity uses server-side `received_at_ms` from
`jpeg_frame_validated`. One server-wide monotonic clock keeps timestamps
comparable across reconnect sessions.

Metrics include unique/missing/duplicate frames, largest frame-ID gap,
out-of-order frames, largest receiver gap, reconnect boundary frames, and
skipped client-timeline frames.

## Per-run result

Schema version 1 contains:

```text
schema_version
strategy
media_run_id
successful
analysis_errors
identity
frames
timing
continuity
```

A result is successful when the media run completed, has a final frame
boundary, and contains no analysis errors. Frame loss is a measured outcome,
not an analysis failure.

```bash
python3 -m scripts.mininet.recovery_result   --client-log /path/to/client.log   --server-log /path/to/server.log   --output /path/to/result.json
```

## Summary CSV

```bash
python3 -m scripts.mininet.recovery_summary   /path/to/migrate/result.json   /path/to/reconnect/result.json   --output /path/to/summary.csv
```

## Repeated runner

Supported placeholders are `{strategy}`, `{trial_index}`, `{trial_id}`, and
`{trial_dir}`.

```bash
sudo python3 -m scripts.mininet.recovery_experiment   --output-root artifacts/recovery-experiment   --repetitions 10   --scenario-command   '["python3","scripts/mininet/migration_demo.py","--noninteractive","--preset","health-sustained","--recovery-strategy","{strategy}","--no-quiet-media-logs","--log-dir","{trial_dir}"]'
```

The runner executes without a shell, captures output, handles timeouts and
failures, analyzes completed logs, and produces a combined CSV.

`results/recovery-experiment-01/` contains 20 result JSON files and
`summary.csv`. Raw logs and packet captures are not committed.

## Limitations

Client action timing and server receive timing use separate process-relative
clocks. Action durations are strategy-specific. Immediate old-session
termination is not required. The current dataset uses one deterministic
topology and failure pattern.
