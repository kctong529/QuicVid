# Mininet QUIC Migration Demo

This document describes the reproducible Mininet setup used to demonstrate
QuicVid QUIC connection migration across two network paths while preserving the
same media session.

The demo uses a fixed server service address and two client-side source
addresses:

```text
Path A: 10.0.1.2
Path B: 10.0.2.2
Server: 10.0.0.1
```

During a run, the client initially sends media through Path A, then explicitly
rebinds the active Quinn endpoint to Path B. The same QUIC connection and
logical QuicVid session continue after the path change.

## Topology

```text
                         Server
                    service: 10.0.0.1
                           (lo)
                         /      \
                        /        \
            10.0.3.2 /            \ 10.0.4.2
              server-eth0        server-eth1
                    |                |
                 10.0.3.0/24     10.0.4.0/24
                    |                |
                 10.0.3.1        10.0.4.1
                     r1              r2
                 10.0.1.1        10.0.2.1
                    |                |
                 10.0.1.0/24     10.0.2.0/24
                    |                |
                 10.0.1.2        10.0.2.2
              client-eth0        client-eth1
                        \          /
                         \        /
                           Client
```

The server uses `10.0.0.1/32` on its loopback interface as a stable service
address. This keeps the remote QUIC endpoint unchanged while the client moves
between the two paths.

Source-based policy routing ensures:

```text
source 10.0.1.2 -> Path A via r1
source 10.0.2.2 -> Path B via r2
```

The topology implementation is in:

```text
scripts/mininet/dual_path.py
```

The migration demo launcher is in:

```text
scripts/mininet/migration_demo.py
```

## Requirements

The demo is intended to run on Linux with:

* Mininet
* Open vSwitch
* `iproute2`
* `xterm`
* `tcpdump`
* Rust toolchain

The tested Mininet setup uses an Open vSwitch bridge without a controller.

A basic Mininet installation can be checked with:

```bash
sudo mn --switch ovsbr --test pingall
```

A working installation should report zero packet loss.

## Build QuicVid

From the repository root:

```bash
cargo build --release --manifest-path quic-vid/Cargo.toml
```

The migration launcher expects the resulting binary at:

```text
quic-vid/target/release/quic-vid
```

## Run the migration demo

Clean any previous Mininet state first:

```bash
sudo mn -c
```

Then start one of the predefined demo presets.

### Preview preset

```bash
sudo python3 scripts/mininet/migration_demo.py --preset preview
```

The preview preset is intended for the visual end-to-end demonstration:

```text
FPS:                 30
Duration:            5 s
Initial path:        10.0.1.2 / Path A
Rebind target:       10.0.2.2 / Path B
Rebind after:        2.5 s
Preview:             enabled
```

This gives approximately half of the run on each path:

```text
0.0 s                         2.5 s                         5.0 s
 |-----------------------------|-----------------------------|
          Path A                        Path B
                           migration
```

### Diagnostic preset

```bash
sudo python3 scripts/mininet/migration_demo.py --preset diagnostic
```

The diagnostic preset makes the migration easy to identify in logs and packet
captures:

```text
FPS:                 1
Duration:            2 s
Initial path:        10.0.1.2 / Path A
Rebind target:       10.0.2.2 / Path B
Rebind after:        0.5 s
Preview:             disabled
```

The expected sequence is approximately:

```text
frame 0 -> Path A
          |
          | rebind at 0.5 s
          v
frame 1 -> Path B
```

## Demo terminals

The launcher creates and validates the Mininet topology and then opens four
preconfigured xterm windows:

```text
QuicVid Server
QuicVid Client
Path A - r1
Path B - r2
```

### QuicVid Server

The server starts automatically on the stable service address:

```bash
quic-vid server --listen 10.0.0.1:4433
```

For the preview preset, `--preview` is added automatically.

### QuicVid Client

The client terminal displays the resolved command but waits for Enter before
starting the run.

The migration command is equivalent to:

```bash
quic-vid client \
  --connect 10.0.0.1:4433 \
  --bind 10.0.1.2:0 \
  --rebind 10.0.2.2:0 \
  --rebind-after-seconds <trigger> \
  --fps <fps> \
  --duration-seconds <duration>
```

This allows the server and packet capture windows to be ready before media
transmission begins.

### Path A capture

The `r1` terminal runs:

```bash
tcpdump -ni r1-eth0 udp port 4433
```

This captures traffic on the initial path.

### Path B capture

The `r2` terminal runs:

```bash
tcpdump -ni r2-eth0 udp port 4433
```

This captures traffic after migration.

## Expected migration behavior

Before migration:

```text
client
10.0.1.2
   |
   v
  r1
   |
   v
server
10.0.0.1
```

After migration:

```text
client
10.0.2.2
   |
   v
  r2
   |
   v
server
10.0.0.1
```

The important property is that the server address remains:

```text
10.0.0.1:4433
```

Only the client's local UDP path changes.

The client logs the migration request and successful endpoint rebind, including
the old and new local addresses and the Quinn connection identity.

A typical transition looks like:

```text
old_local=10.0.1.2:<port>
new_local=10.0.2.2:<port>
```

## What to verify

A successful migration run should show all of the following:

1. media is initially sent through Path A;
2. `r1` sees UDP traffic before migration;
3. the configured rebind trigger occurs while the media run is active;
4. the client rebinds from `10.0.1.2` to `10.0.2.2`;
5. subsequent UDP traffic appears on Path B through `r2`;
6. the Quinn connection remains the same;
7. the logical QuicVid session UUID remains the same;
8. no second `HELLO` session setup occurs;
9. media continues after migration;
10. the normal end-of-run `DONE` / `DONE_OK` exchange completes.

For the preview preset, the displayed frame counter should continue advancing
across the migration without an obvious session restart or prolonged freeze.

The preview demo is intended as a qualitative continuity demonstration.
Quantitative disruption and frame-loss measurements are evaluated separately.

## Custom configuration

Preset values can be overridden from the command line.

For example:

```bash
sudo python3 scripts/mininet/migration_demo.py \
  --preset preview \
  --fps 10 \
  --duration-seconds 8 \
  --rebind-after-seconds 3
```

Available overrides include:

```text
--fps
--duration-seconds
--rebind
--rebind-after-seconds
--preview
--no-preview
```

For example, preview can be disabled while retaining the other preview preset
parameters:

```bash
sudo python3 scripts/mininet/migration_demo.py \
  --preset preview \
  --no-preview
```

The migration trigger must be greater than zero and smaller than the total run
duration.

## Verify the topology separately

The dual-path topology can also be started without the migration launcher:

```bash
sudo mn -c
sudo python3 scripts/mininet/dual_path.py
```

The script performs startup connectivity checks before opening the Mininet CLI.

The expected source-based routes can be inspected with:

```bash
client ip route get 10.0.0.1 from 10.0.1.2
```

Expected Path A result:

```text
10.0.0.1 via 10.0.1.1 dev client-eth0 table 101
```

And:

```bash
client ip route get 10.0.0.1 from 10.0.2.2
```

Expected Path B result:

```text
10.0.0.1 via 10.0.2.1 dev client-eth1 table 102
```

Connectivity can also be checked explicitly:

```bash
client ping -I 10.0.1.2 -c 3 10.0.0.1
client ping -I 10.0.2.2 -c 3 10.0.0.1
```

Both should complete without packet loss under the default topology.

## Routing configuration

The client uses two policy-routing tables.

Path A:

```text
source: 10.0.1.2
table:  101
gateway: 10.0.1.1
device: client-eth0
```

Path B:

```text
source: 10.0.2.2
table:  102
gateway: 10.0.2.1
device: client-eth1
```

The server has return routes for both client networks:

```text
10.0.1.0/24 via 10.0.3.1 dev server-eth0
10.0.2.0/24 via 10.0.4.1 dev server-eth1
```

The routers have routes to the stable server service address:

```text
r1: 10.0.0.1/32 via 10.0.3.2
r2: 10.0.0.1/32 via 10.0.4.2
```

Reverse-path filtering is disabled on the involved Mininet nodes so that the
multi-homed topology and source-specific routing behave deterministically.

## Clean up

After the demo, the launcher stops the Mininet network.

If Mininet state remains after an interrupted run, clean it with:

```bash
sudo mn -c
```

## Scope

This demo demonstrates controlled client-side QUIC connection migration.

It intentionally does not implement:

* automatic network-change detection;
* Wi-Fi or NetworkManager-triggered migration;
* path-quality-based migration decisions;
* migration fallback logic;
* physical wireless handover;
* repeated statistical evaluation.

Those concerns are outside the scope of the controlled migration demo.

The purpose of this setup is to provide a deterministic and reproducible
environment in which the path change is known in advance and the continuity of
the same QUIC media session can be directly observed.
