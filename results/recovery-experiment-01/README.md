# Recovery Experiment 01

Comparative evaluation of QUIC connection migration and proactive QUIC
reconnect after a sustained Path A failure.

## Experiment configuration

- Date: 2026-07-29
- Trials: 10 migration and 10 reconnect
- Execution order: interleaved by repetition
- Frame rate: 10 fps
- Media duration: 6 seconds
- Expected frames per run: 60
- Path failure: sustained Path A impairment after 2 seconds
- Suspect threshold: 250 ms
- Challenge threshold: 500 ms
- Topology, detector thresholds, impairment timing, and logging were identical
  across strategies
- Only `recovery_strategy` changed between `migrate` and `reconnect`

Command used:

```bash
sudo python3 -m scripts.mininet.recovery_experiment   --output-root artifacts/recovery-experiment   --repetitions 10   --scenario-command   '["python3","scripts/mininet/migration_demo.py","--noninteractive","--preset","health-sustained","--recovery-strategy","{strategy}","--no-quiet-media-logs","--log-dir","{trial_dir}"]'
```

## Results

| Metric | Migration | Reconnect |
|---|---:|---:|
| Successful runs | 10/10 | 10/10 |
| Median largest receive gap | 937.4 ms | 802.0 ms |
| Mean largest receive gap | 973.3 ms | 830.2 ms |
| Median missing frames | 2 | 7 |
| Mean missing frames | 2.4 | 7.6 |
| Median recovery-action duration | 156 ms | 1 ms |
| Duplicate frames | 0 | 0 |
| Out-of-order frames | 0 | 0 |

Reconnect restored receiver activity sooner, while migration preserved more
media frames and retained one QUIC connection and session. Both strategies
recovered successfully in all tested runs.

The full interpretation, descriptive statistics, architecture comparison, and
limitations are documented in:

```text
analysis/summary.md
```

## Final figures

```text
analysis/plots/
├── receive-gap-histogram.png
└── missing-frames-frequency.png
```

![Distribution of the largest receiver-observed frame gap across ten migration and ten reconnect trials.](analysis/plots/receive-gap-histogram.png)

*Figure 1. Distribution of the largest receiver-observed frame gap across ten
migration and ten reconnect trials. Reconnect produced shorter receiver-visible
interruptions in the tested Mininet scenario, while one migration trial showed
a substantially larger outlier.*

![Frequency of global missing-frame counts across ten migration and ten reconnect trials.](analysis/plots/missing-frames-frequency.png)

*Figure 2. Frequency of global missing-frame counts across ten trials per
strategy. Migration usually lost two frames, whereas reconnect most commonly
lost seven or eight frames.*

The plots are generated from `summary.csv`, not from manually entered values.

## Metric interpretation

`largest_receive_gap_ms` is the primary cross-strategy interruption metric.

`recovery_action_duration_ms` is diagnostic rather than directly comparable:
migration completion waits for ACK-confirmed progress after endpoint rebind,
whereas reconnect completion records replacement-session establishment.

Reconnect loss is calculated globally across S1 and S2. The replacement
session's local missing count is not used as the media-run-wide loss result.

## Outlier

`migrate-003` had the largest migration receive gap:

- largest receive gap: 1268.4 ms
- recovery-action duration: 439 ms
- missing frames: 5

The trial remained valid and is retained in the statistics and figures.

## Demo

A preview-mode demonstration is available at:

[Final recovery demo](../final-demo/simultaneously-preview.mp4)

## Files

- `summary.csv`: one flat comparison row per trial
- `runs/*.json`: structured per-run results
- `analysis/summary.csv`: aggregate strategy statistics
- `analysis/summary.md`: interpreted comparison
- `analysis/plots/*.png`: generated final figures

Raw client/server logs and packet captures are intentionally excluded from Git
because they are large and reproducible.

## Reproduce

See:

- [`../../docs/mininet-migration.md`](../../docs/mininet-migration.md)
- [`../../docs/recovery-analysis.md`](../../docs/recovery-analysis.md)

The committed records provide compact evidence for the final report. The
measured findings apply to this controlled Mininet configuration and should not
be generalized directly to physical wireless networks.
