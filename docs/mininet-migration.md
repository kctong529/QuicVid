# Mininet QUIC Migration Demo

This document describes the reproducible Mininet setup used to demonstrate
QuicVid QUIC connection migration across two network paths while preserving the
same media session, and to exercise the automatic path-health logic introduced
for Milestone 4.

The demo uses a fixed server service address and two client-side source
addresses:

```text
Path A: 10.0.1.2
Path B: 10.0.2.2
Server: 10.0.0.1
```

Controlled migration presets initially send media through Path A, then explicitly
rebind the active Quinn endpoint to Path B. The same QUIC connection and logical
QuicVid session continue after the path change.

Automatic path-health presets keep the client on Path A and introduce a
reproducible impairment. A transient outage verifies recovery on the original
path, while sustained degradation drives automatic discovery, selection, and
migration to Path B.

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

### Health-transient preset

```bash
sudo python3 scripts/mininet/migration_demo.py --preset health-transient
```

This preset exercises automatic path-health recovery from a temporary outage:

```text
FPS:                 10
Duration:            4 s
Initial path:        10.0.1.2 / Path A
Automatic health:    enabled
Suspect after:       250 ms
Challenge after:     1000 ms
Path A impairment:   starts at 2.0 s
Impairment duration: 350 ms
Preview:             disabled
```

The launcher temporarily takes `r1-eth0` down and restores it after 350 ms.
The expected controller sequence is:

```text
Healthy
  |
  | ACK progress stops
  v
Suspect
  |
  | ACK progress resumes before challenge threshold
  v
Healthy
```

A successful run should emit both:

```text
event=path_health status=suspect ...
event=path_health status=recovered ...
```

with matching migration-state transitions:

```text
Healthy -> Suspect -> Healthy
```

The `1000 ms` challenge threshold is deliberately longer than the transient
recovery interval. Shorter values were observed to request a challenge before
QUIC ACK progress had resumed, even after the link itself had already been
restored. This trade-off is evaluated quantitatively later.

### Health-sustained preset

```bash
sudo python3 scripts/mininet/migration_demo.py --preset health-sustained
```

This preset exercises the complete automatic migration path after persistent
degradation:

```text
FPS:                 10
Duration:            6 s
Initial path:        10.0.1.2 / Path A
Automatic migration: enabled
Suspect after:       250 ms
Challenge after:     500 ms
Path A impairment:   starts at 2.0 s
Impairment duration: sustained
Preview:             disabled
```

The launcher takes `r1-eth0` down and leaves it unavailable. The expected
controller sequence is:

```text
Healthy
  |
  | ACK progress stops
  v
Suspect
  |
  | degradation persists
  v
Challenging
  |
  | discover and select 10.0.2.2
  v
Migrating
  |
  | ACK progress resumes after endpoint rebind
  v
Healthy
```

A successful run should emit:

```text
event=path_health status=suspect ...
event=path_health status=challenge_requested ...
event=path_discovery_started ...
event=path_candidate_found interface=client-eth1 candidate_ip=10.0.2.2
event=path_candidate_selected interface=client-eth1 candidate_ip=10.0.2.2
event=automatic_rebind_started ...
event=migration_state ... from=Challenging to=Migrating ...
event=endpoint_rebound mode=automatic ...
event=path_health status=recovered ...
event=migration_state ... from=Migrating to=Healthy ...
event=migration_confirmed ...
```

Automatic mode does not receive a fallback `--rebind` address. QuicVid
enumerates local IPv4 addresses, excludes unsuitable and active addresses,
selects the first remaining candidate in deterministic order, and binds a new
UDP socket for `Endpoint::rebind()`.

A successful rebind call is not treated as proof of recovery. The controller
remains in `Migrating` until new QUIC ACK progress is observed.

## Automatic migration behavior

In the sustained scenario, packet captures should show:

```text
before impairment:
10.0.1.2 -> Path A through r1

after automatic rebind:
10.0.2.2 -> Path B through r2
```

The remote server address remains `10.0.0.1:4433`. Only the client's local UDP
path changes.

The automatic migration preserves:

* the same Quinn connection identity;
* the same QuicVid session UUID;
* the existing media and control session;
* a single initial `HELLO`.

No reconnect or second application handshake occurs. Normal completion should
still reach `DONE`, `DONE_OK`, `jpeg_video_done_acknowledged`, and
`client_stopped`.


## Path-health signal

Automatic path-health monitoring currently uses the cumulative QUIC ACK-frame
counter exposed by Quinn connection statistics as its progress signal.

While media is active, an increasing ACK count indicates ongoing transport-level
progress on the current path. The client polls this value periodically and feeds
it into the path-health monitor.

The configured thresholds have the following meaning:

```text
last observed ACK progress
        |
        | suspect-after-ms
        v
     Suspect
        |
        | challenge-after-ms
        v
   Challenging
```

If ACK progress resumes while the controller is `Suspect` or `Challenging`, the
controller returns to `Healthy`.

`send_datagram()` success is not used as the health signal. A successful call
only means the DATAGRAM was accepted by Quinn for transmission; it does not
prove that the packet crossed the current path or reached the peer.

## Demo terminals

The launcher creates and validates the Mininet topology and opens the standard
preconfigured xterm windows:

```text
QuicVid Server
QuicVid Client
Path A - r1
Path B - r2
```

For `health-transient` and `health-sustained`, it also opens:

```text
Path A Impairment
```

The impairment controller waits for a client-start marker and schedules the Path A
outage relative to the actual client start, rather than relative to launcher or
xterm startup.

### QuicVid Server

The server starts automatically on the stable service address:

```bash
quic-vid server --listen 10.0.0.1:4433
```

For the preview preset, `--preview` is added automatically.

### QuicVid Client

The client terminal displays the resolved command but waits for Enter before
starting the run.

For controlled migration presets, the client command is equivalent to:

```bash
quic-vid client \
  --connect 10.0.0.1:4433 \
  --bind 10.0.1.2:0 \
  --rebind 10.0.2.2:0 \
  --rebind-after-seconds <trigger> \
  --fps <fps> \
  --duration-seconds <duration>
```

For automatic path-health presets, it is equivalent to:

```bash
quic-vid client \
  --connect 10.0.0.1:4433 \
  --bind 10.0.1.2:0 \
  --auto-migrate \
  --suspect-after-ms <threshold> \
  --challenge-after-ms <threshold> \
  --fps <fps> \
  --duration-seconds <duration>
```

Automatic mode deliberately does not receive a `--rebind` target. Reaching
`Challenging` triggers local IPv4 candidate discovery and selection. The client
then rebinds the existing Quinn endpoint to the selected address and waits for
new ACK progress before returning to `Healthy`.

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

For the automatic presets, verify the controller behavior separately:

```text
health-transient:
Healthy -> Suspect -> Healthy

health-sustained:
Healthy -> Suspect -> Challenging -> Migrating -> Healthy
```

The transient preset recovers on Path A without discovery or rebind. The
sustained preset discovers `client-eth1 / 10.0.2.2`, rebinds the existing Quinn
endpoint, moves traffic to Path B, and confirms recovery from resumed ACK
progress.

## Preset logging modes

The launcher applies logging defaults suited to each scenario:

| Preset | Per-frame logs | Per-datagram logs | Intended use |
|---|---:|---:|---|
| `diagnostic` | shown | shown | Full low-rate diagnostic output |
| `preview` | shown | suppressed | Preview with reduced chunk-level noise |
| `health-transient` | suppressed | suppressed | Focus on path-health recovery |
| `health-sustained` | suppressed | suppressed | Focus on automatic migration |

These are preset defaults rather than fixed modes. They can be overridden with
the launcher options listed below. For example, this keeps per-frame output while
suppressing per-datagram output:

```bash
sudo python3 scripts/mininet/migration_demo.py \
  --preset health-sustained \
  --no-quiet-media-logs \
  --quiet-datagram-logs
```

Migration, path-health, discovery, connection, and final summary events remain
visible when media logs are suppressed.


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
--auto-migrate
--no-auto-migrate
--suspect-after-ms
--challenge-after-ms
--impair-after-seconds
--impair-duration-seconds
--quiet-media-logs
--no-quiet-media-logs
--quiet-datagram-logs
--no-quiet-datagram-logs
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

## Scope and current limitations

This setup now supports three reproducible Mininet experiments:

* controlled client-side QUIC connection migration from Path A to Path B;
* transient path degradation followed by recovery on Path A;
* sustained Path A failure followed by automatic discovery and migration to
  Path B.

The automatic migration path is:

```text
Healthy -> Suspect -> Challenging -> Migrating -> Healthy
```

Current limitations are:

* automatic discovery supports IPv4 addresses only;
* candidates are filtered and ordered deterministically but are not ranked by
  reachability, interface type, RTT, bandwidth, or cost;
* the first remaining candidate is selected;
* no application-layer QUIC `PATH_CHALLENGE` or `PATH_RESPONSE` implementation
  is used;
* Quinn handles transport-level migration and path validation internally;
* if no candidate is found, the controller remains in `Challenging`;
* there is no candidate-discovery retry policy;
* a rebind failure is logged and terminates the client;
* there is no dedicated migration-confirmation timeout, so Quinn's connection
  timeout remains the final failure mechanism;
* the automatic scenario is currently validated in the controlled Mininet
  topology;
* Wi-Fi or NetworkManager-triggered migration, physical wireless handover,
  repeated statistical evaluation, threshold sweeps, reconnect baselines, and
  TCP baselines remain outside this setup.

The purpose of this setup is to provide a deterministic environment for
controlled migration, automatic degradation detection, alternative-address
discovery, and recovery confirmation before quantitative evaluation.
