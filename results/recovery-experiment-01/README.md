# Recovery Experiment 01

Comparative evaluation of QUIC connection migration and QUIC reconnect after a sustained Path A failure.

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
- Topology, detector thresholds, impairment timing, and logging were identical across strategies
- Only `recovery_strategy` changed between `migrate` and `reconnect`

Command used:

```bash
sudo python3 -m scripts.mininet.recovery_experiment \
  --output-root artifacts/recovery-experiment \
  --repetitions 10 \
  --scenario-command \
  '["python3","scripts/mininet/migration_demo.py","--noninteractive","--preset","health-sustained","--recovery-strategy","{strategy}","--no-quiet-media-logs","--log-dir","{trial_dir}"]'
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

Reconnect restored receiver activity sooner, while migration preserved more media frames and retained one QUIC connection and session. Both strategies recovered successfully in all tested runs.

The strategy-specific action duration is diagnostic rather than a directly comparable end-to-end interruption metric. Migration completion waits for ACK-confirmed progress after endpoint rebind, whereas reconnect completion records replacement-session establishment. Receiver-side frame gaps are the primary cross-strategy interruption metric.

### Outlier

`migrate-003` had the largest migration receive gap:

- largest receive gap: 1268.4 ms
- recovery-action duration: 439 ms
- missing frames: 5

The remaining migration trials had receive gaps between 935.3 and 963.8 ms.

## Files

- `summary.csv`: one flat comparison row per trial
- `runs/*.json`: structured per-run results used to build the summary

Raw client/server logs and packet captures are intentionally excluded from Git because they are large and reproducible from the command above.

## Reproduce

See [`../../docs/mininet-migration.md`](../../docs/mininet-migration.md) and
[`../../docs/recovery-analysis.md`](../../docs/recovery-analysis.md).

The committed JSON records are compact evidence for Epic 5.4. The descriptive
values above apply only to this controlled Mininet configuration.
