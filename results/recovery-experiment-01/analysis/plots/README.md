# Recovery plot alternatives

Valid plotted trials: 20.
Excluded trials: 0.

- `receive-gap-jittered.png`: recommended primary interruption plot;
- `missing-frames-frequency.png`: recommended frame-loss plot;
- `receive-gap-histogram.png`: supporting histogram using 25 ms bins.

The jitter is deterministic and is used only to reveal overlapping observations. It does not modify the measured y-values.

The missing-frame plot uses exact discrete counts because missing frames are integer-valued.
