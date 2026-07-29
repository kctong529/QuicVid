# Recovery comparison

Generated from the flat Epic 5.3 trial summary.

## Experiment validity

| Strategy | Total trials | Valid trials | Invalid trials | Successful trials | Success rate | Analysis errors |
|---|---:|---:|---:|---:|---:|---:|
| Migrate | 10 | 10 | 0 | 10 | 100% | 0 |
| Reconnect | 10 | 10 | 0 | 10 | 100% | 0 |

## Main comparison

| Strategy | Valid runs | Success rate | Mean receive gap | Median receive gap | Mean missing frames | Median missing frames |
|---|---:|---:|---:|---:|---:|---:|
| Migrate | 10 | 100% | 973.305 ms | 937.373 ms | 2.400 | 2 |
| Reconnect | 10 | 100% | 830.204 ms | 802.026 ms | 7.600 | 7 |

## Receiver-visible interruption

The primary cross-strategy interruption metric is `largest_receive_gap_ms`.

| Strategy | Count | Min | Mean | Median | Max | Sample stddev |
|---|---:|---:|---:|---:|---:|---:|
| Migrate | 10 | 935.329 ms | 973.305 ms | 937.373 ms | 1268.359 ms | 104.024 ms |
| Reconnect | 10 | 798.226 ms | 830.204 ms | 802.026 ms | 900.096 ms | 47.748 ms |

## Frame preservation

| Strategy | Missing min | Missing mean | Missing median | Missing max | Missing stddev | Mean received unique |
|---|---:|---:|---:|---:|---:|---:|
| Migrate | 2 | 2.400 | 2 | 5 | 0.966 | 57.600 |
| Reconnect | 7 | 7.600 | 7 | 10 | 0.966 | 52.400 |

## Transport identity

| Strategy | Mean sessions | Mean connections | Duplicate frames | Out-of-order frames |
|---|---:|---:|---:|---:|
| Migrate | 1 | 1 | 0 | 0 |
| Reconnect | 2 | 2 | 0 | 0 |

## Strategy-specific action timing

`recovery_action_duration_ms` is diagnostic. Migration and reconnect use different completion events, so this metric is not treated as an equivalent end-to-end interruption measurement.

| Strategy | Count | Min | Mean | Median | Max | Sample stddev |
|---|---:|---:|---:|---:|---:|---:|
| Migrate | 10 | 155 ms | 188.900 ms | 156 ms | 439 ms | 88.456 ms |
| Reconnect | 10 | 1 ms | 1.100 ms | 1 ms | 2 ms | 0.316 ms |

## Notes and limitations

- Statistics use only successful rows with zero analysis errors.
- Standard deviation is the sample standard deviation.
- Invalid or malformed rows are reported rather than silently included.
- Results describe one controlled Mininet configuration and should not be generalized to physical wireless networks without further experiments.
- Individual observations and plots should accompany these aggregates because the trial count is small.

## Preliminary observation

- Reconnect median receive gap: 802.026 ms.
- Migration median receive gap: 937.373 ms.
- Migration median missing frames: 2.
- Reconnect median missing frames: 7.

In this dataset, reconnect restored receiver activity sooner, while migration preserved more media frames and retained the existing transport identity.
