# Recovery analysis and experiment data

## Pipeline

```text
client.log + server.log
        |
        v
recovery_result.py -> result.json
        |
        +--> recovery_summary.py -> trial summary.csv
        |
        +--> recovery_experiment.py -> repeated trials
        |
        +--> recovery_compare.py -> aggregate summary.csv + summary.md
        |
        +--> recovery_plot_alternatives.py -> final plots
```

## Per-run analysis

The parser accepts `event=<name> key=value ...` lines, including prefixes and
quoted values. UUIDs and addresses remain strings. Malformed structured lines
are reported; ordinary output is ignored.

One per-run result records:

- strategy and `media_run_id`;
- final frame boundary;
- QuicVid session IDs;
- Quinn connection stable IDs;
- HELLO sessions;
- initial and recovered addresses;
- completion state;
- received, missing, and duplicate frames;
- receiver-side continuity;
- strategy-specific action timing.

Expected identity:

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

The S2-local receive summary is not the media-run-wide result.

## Timing and continuity

```text
migration:
    automatic_rebind_started -> migration_confirmed

reconnect:
    reconnect_started -> reconnect_completed
```

These completion events have different semantics. Action duration is therefore
a diagnostic metric.

Receiver continuity uses server-side `received_at_ms` from
`jpeg_frame_validated`. One server-wide monotonic clock keeps timestamps
comparable across reconnect sessions.

`largest_receive_gap_ms` is the primary cross-strategy interruption metric.

## Generate one result

```bash
python3 -m scripts.mininet.recovery_result   --client-log /path/to/client.log   --server-log /path/to/server.log   --output /path/to/result.json
```

## Generate trial CSV

```bash
python3 -m scripts.mininet.recovery_summary   /path/to/migrate/result.json   /path/to/reconnect/result.json   --output /path/to/summary.csv
```

## Run repeated trials

```bash
sudo python3 -m scripts.mininet.recovery_experiment   --output-root artifacts/recovery-experiment   --repetitions 10   --scenario-command   '["python3","scripts/mininet/migration_demo.py","--noninteractive","--preset","health-sustained","--recovery-strategy","{strategy}","--no-quiet-media-logs","--log-dir","{trial_dir}"]'
```

## Generate aggregate statistics

```bash
python3 -m scripts.mininet.recovery_compare   --input results/recovery-experiment-01/summary.csv   --output-dir results/recovery-experiment-01/analysis
```

For each strategy and metric, the tool reports:

```text
count
minimum
mean
median
maximum
sample standard deviation
```

Unsuccessful rows, malformed rows, and rows with analysis errors are reported
and excluded explicitly.

## Generate final figures

```bash
python3 -m scripts.mininet.recovery_plot_alternatives   --input results/recovery-experiment-01/summary.csv   --output-dir results/recovery-experiment-01/analysis/plots   --histogram-bin-width-ms 50
```

Final report figures:

```text
receive-gap-histogram.png
missing-frames-frequency.png
```

The receive-gap histogram uses the same fixed 50 ms bins for both strategies.
The missing-frame figure shows exact discrete trial frequencies.

## Current interpretation

Across ten trials per strategy:

- both strategies completed every run;
- reconnect had a shorter receiver-visible interruption;
- migration lost fewer frames;
- migration preserved one connection and session;
- reconnect used two connections and sessions;
- both preserved one logical media run and continuous frame timeline.

The complete interpretation is in:

```text
results/recovery-experiment-01/analysis/summary.md
```

## Limitations

- ten trials per strategy;
- deterministic Mininet topology;
- one sustained failure pattern;
- one frame rate, duration, and threshold configuration;
- separate client-relative and server-relative clocks;
- no physical Wi-Fi handover;
- no NetworkManager trigger;
- no TCP or timeout-reconnect baseline;
- no repeated recovery within one run;
- no parameter sweep.

The results are descriptive evidence for this controlled setup, not a general
performance claim for real wireless networks.
