# QuicVid

**A Quinn-based video-call prototype for demonstrating robust call continuity during network changes**

QuicVid is a bachelor final project at Aalto University. The engineering problem is that a video-call application is normally built on top of a transport connection that assumes the endpoint's network address remains stable. When a laptop changes network, a wireless link is interrupted, a NAT binding changes, or a client moves to a different path, the application may lose the active media session even though the user still thinks they are in the same call.

This project asks a concrete engineering question:

> Can a desktop video-call prototype use QUIC connection migration to keep an active media session alive across a controlled network/address change, with less visible disruption than a simple baseline that has to reconnect or loses session continuity?

The scope is therefore not “build a production Zoom replacement.” The scope is a focused product prototype with a clear demonstration:

1. run a video-call app as separate client and server instances;
2. send live or repeatable video frames over Quinn/QUIC;
3. trigger client-side migration while the call is active;
4. show that the QUIC session remains the same application call when migration succeeds;
5. compare the result with a simple non-migrating baseline;
6. log frame disruption, peer address changes, and reconnect/session behavior.

For the detailed project scope, milestone breakdown, runtime modes, and definition of done, see [`PLAN.md`](PLAN.md).

## Project background

This project started in September with a more ambitious plan: to build a QUIC-based video-call system using quiche, Qt, FFmpeg, mobile-style handover experiments, commercial video-app comparisons, and possibly user-study-style evaluation.

That original plan helped define the motivation, but it did not become the actual implementation path. In practice, the work from the earlier phase focused mainly on understanding QUIC migration and building smaller experiments around it. The most useful results from that phase are the Quinn-based `quinn-ping` and `mouse-coordinates` prototypes, which already demonstrate connection setup, datagram traffic, endpoint rebinding, and session continuity behavior.

The project is now being restarted from that more realistic foundation. The current direction is to turn the working Quinn experiments into a desktop video-call prototype that clearly demonstrates the benefit of QUIC for robust call continuity during controlled network changes.

## Academic context

- **Student:** Tong Ki Chun
- **Advisor:** Pasi Sarolahti
- **Institution:** Aalto University, Department of Electrical Engineering
- **Type:** Bachelor's final project

## Engineering problem and project goal

The project focuses on a transport-level robustness problem in video-call applications. Video calls are long-lived interactive sessions, but common transport designs bind connection state to the currently used network address and port. When that address changes, the media application may need to reconnect, recreate state, or recover at the application layer. For a user, that appears as a frozen video, dropped call, or long recovery delay.

QUIC offers a different design point: the connection can be identified by connection IDs rather than only by the IP/port tuple, and a new path can be validated while the connection remains logically the same session. QuicVid uses this property as the main engineering idea.

The goal is to build a runnable prototype that makes the benefit visible:

- a live or repeatable video stream is sent over Quinn/QUIC;
- the client changes its local network path or UDP binding during the call;
- the server observes the peer address/port change;
- the application keeps the same call session when migration succeeds;
- logs show the frame disruption around the migration event;
- a simple baseline shows the cost of not having QUIC migration support.

The expected achievement is a **controlled proof of product feasibility**: QuicVid should demonstrate that QUIC migration can reduce application-visible disruption for a video-call workload under the tested conditions. It does not claim to solve public Internet NAT traversal, mobile handover, conferencing, or production video quality.

## Current implementation direction

The active implementation path is:

- **Language:** Rust
- **QUIC library:** Quinn
- **Architecture:** separate client and server app instances
- **Media transport:** QUIC datagrams
- **Control messages:** QUIC streams
- **Migration mechanism:** Quinn endpoint rebinding and controlled network experiments
- **Target:** desktop application

The stack is intentionally revised from the old plan:

| Area | Current decision |
|---|---|
| QUIC implementation | Quinn is required for all new product work |
| quiche | Legacy/background only |
| GUI toolkit | Not fixed; Rust-native GUI such as `egui`/`eframe` or `iced` is preferred |
| Qt | Old plan item; optional, not required |
| Video pipeline | Start simple with test-pattern/raw/resized/JPEG-style frames before full codec integration |
| FFmpeg/GStreamer | Optional later backend if needed for quality or bandwidth control |
| Audio | Important follow-up after video + migration works |

Older quiche work remains in the repository as legacy/background material. New product work should use Quinn. Qt and FFmpeg were part of the original aspirational plan, but they are not required for the first complete prototype unless they become the fastest practical route to a working demo.

## Important note about the old plan

Earlier project documents should be read as planning history, not as the current implementation contract. The revised project should be judged by whether it demonstrates the QUIC robustness benefit in a working video-call prototype, not by whether it follows the old Qt/FFmpeg/quiche stack exactly.

## Current repository status

The repository currently contains experimental work and project history rather than a finished app.

### `quinn-ping/`

A basic Quinn client/server ping experiment.

It demonstrates:

- QUIC connection setup with Quinn;
- bidirectional stream communication;
- repeated ping/pong messages;
- endpoint rebinding;
- stable connection behavior across address/port changes;
- RTT and application-latency logging.

This is the current sanity check for basic Quinn migration behavior.

### `mouse-coordinates/`

A Quinn datagram experiment that streams mouse movement data.

It demonstrates:

- real-time telemetry over QUIC datagrams;
- low-latency fire-and-forget updates;
- server-side terminal visualization;
- session persistence during rebinding/migration;
- a traffic pattern closer to video media than ping/pong.

This is the closest existing stepping stone toward video frame transport.

### quiche files and notes

The repository includes older C/quiche experiments and documentation. These helped explore QUIC migration concepts, path validation, connection IDs, and lower-level API behavior.

They are now legacy context only. The main QuicVid product should not depend on quiche.

### baseline-study documents

The old baseline-study documents are useful as motivation, but the full commercial-app/user-study plan is no longer the main evaluation scope.

The revised project should use a smaller controlled baseline comparison against the QuicVid prototype.

## Runtime architecture

QuicVid should run as two app instances: one server and one client.

```text
Client app instance                         Server app instance
-------------------                         -------------------
GUI                                        GUI or receiver view
Camera capture                             Camera capture optional
Video encoder                              Video encoder optional
Video sender  ───── QUIC datagrams ─────▶  Video receiver
Video receiver ◀──── QUIC datagrams ─────  Video sender optional
Control task  ───── QUIC stream ────────▶  Control task
Migration trigger                          Migration observer
Logs                                       Logs
```

### Server role

The server app:

- listens on a UDP port;
- accepts Quinn connections;
- receives video datagrams;
- displays remote video;
- handles reliable control messages;
- logs peer address changes, frame delivery, and migration events;
- optionally sends its own video back to the client in bidirectional mode.

### Client role

The client app:

- connects to the server;
- captures local camera video;
- sends video over QUIC datagrams;
- handles reliable control messages;
- triggers migration during the robustness demo;
- logs frame sending and migration timing;
- optionally receives and displays server video in bidirectional mode.

## Execution modes

The project should support three execution modes.

### 1. Local two-process mode

Both server and client run on the same machine.

Example target commands:

```bash
cargo run --bin quicvid -- --mode server --listen 127.0.0.1:4433
cargo run --bin quicvid -- --mode client --connect 127.0.0.1:4433
```

Purpose:

- fastest development loop;
- validates GUI, media capture, media rendering, Quinn connection, packet format, and logging;
- allows initial migration testing through local UDP socket rebinding.

Expected result:

- client captures video;
- server displays remote video;
- migration button or auto-migration flag changes the client endpoint binding;
- server logs a peer address/port change;
- the QUIC call remains active if migration succeeds.

### 2. Two-host LAN mode

Server and client run on different reachable machines.

Example target commands:

```bash
# Host A
cargo run --bin quicvid -- --mode server --listen 0.0.0.0:4433

# Host B
cargo run --bin quicvid -- --mode client --connect <server-ip>:4433
```

Purpose:

- demonstrates the app as an actual two-machine video-call prototype;
- avoids relying only on loopback behavior;
- supports a more convincing product demo.

Expected result:

- video is sent from one host to the other;
- optionally video is bidirectional;
- controlled migration/rebinding can be triggered during the call;
- the server observes address/port changes and logs the result.

Limitations:

- no public Internet NAT traversal;
- no account system;
- both machines must be directly reachable.

### 3. Mininet/evaluation mode

The client and server run inside a controlled Mininet topology.

Simple target topology:

```text
h1/client ─── s1 ─── h2/server
```

Possible later topology:

```text
             path A
h1/client ─────────── h2/server
     │                  ▲
     └──── path B ──────┘
```

Purpose:

- repeatable robustness testing;
- scripted migration/disruption events;
- controlled comparison between QUIC and baseline behavior;
- evidence for the final report.

This mode should be implemented after local two-process and two-host modes are working.

## GUI and media backend policy

QuicVid should not block on Qt or FFmpeg unless they are clearly the fastest route to a working prototype.

For the GUI, prefer a Rust-native toolkit that integrates cleanly with async Quinn tasks. `egui`/`eframe` or `iced` are good candidates. The GUI should stay thin and communicate with transport/media workers through channels.

For media, use a staged path:

1. test-pattern frames for repeatable network testing;
2. live camera preview;
3. simple transmitted frames, such as raw resized frames or JPEG-compressed frames;
4. datagram chunking and frame reassembly if needed;
5. FFmpeg or GStreamer only if simple frame transport is not enough for the robustness demo.

This keeps the project focused on the core claim: QUIC can help preserve a video-call session during controlled network migration.

## Transport design

QuicVid should use two kinds of QUIC data flow.

### QUIC streams

Reliable QUIC streams are used for control messages:

- call setup;
- call shutdown;
- media settings;
- migration/debug messages;
- graceful teardown.

### QUIC datagrams

QUIC datagrams are used for media:

- video frame packets or chunks;
- optional audio packets;
- low-latency telemetry.

Datagrams are preferred for media because late video/audio data is often less useful than fresh data. This also builds naturally on the existing `mouse-coordinates` datagram prototype.

## Migration model

The first migration demo should be client-side.

The client triggers migration during an active video call by rebinding the Quinn endpoint to a new UDP socket/local port. Later, where possible, this can be extended to binding to a different local interface or address.

The expected observation is:

- the server sees the client's remote address/port change;
- QUIC keeps the connection associated with the same session;
- video continues or resumes without a full application reconnect;
- logs show frame disruption around the migration event.

## Baseline model

The baseline exists to show what QUIC adds.

The baseline should use the same or similar video workload but without QUIC migration support.

Possible baseline choices:

1. TCP video stream;
2. UDP video stream with manual application-level session token;
3. reconnect-based video sender.

The minimum requirement is one clear baseline where the same disruption causes a visible interruption, reconnect, or new session.

## Definition of done

The project is complete when:

1. QuicVid runs as a desktop application.
2. The app uses Quinn as the active QUIC implementation.
3. The app does not require quiche, Qt, or FFmpeg unless they were explicitly selected as active implementation dependencies.
4. The app can run in server mode.
5. The app can run in client mode.
6. Server and client can run as separate processes.
7. Local two-process mode is documented.
8. Two-host mode is documented or has a clear blocker.
9. The client captures live camera video.
10. The receiver displays remote video.
11. Video data is transported over Quinn/QUIC.
12. A controlled migration can be triggered during an active call.
13. The app shows or logs migration state.
14. The QUIC session attempts to continue after migration.
15. A baseline demo without QUIC migration support exists.
16. Logs or summary output compare QUIC and baseline behavior.
17. The README explains what the demo proves and does not prove.
18. Limitations are documented clearly.

## Minimum successful demo

The minimum successful final demo has two runs.

### Baseline run

Run a simple non-QUIC-migration video transport.

Then trigger a controlled disruption or address/path change.

Show one of:

- video freezes;
- stream stops;
- reconnect is required;
- a new session starts;
- interruption is visibly larger than in the QUIC demo.

### QUIC run

Run QuicVid over Quinn.

Then trigger migration during the active video call.

Show:

- the same call remains active;
- the server observes peer address/port change;
- the app does not silently start a new session;
- video continues or resumes with bounded disruption;
- logs summarize the migration event.

Example target summary:

```text
Experiment: quic-video-migration-001
Migration triggered at: 12.40s
Connection survived: yes
Application reconnect required: no
Peer address changed: yes
Frames sent: 472
Frames received: 459
Frames lost near migration: 8
Estimated visible freeze: 280 ms
```

## Target repository structure

This is the intended future structure, not necessarily the current state.

```text
QuicVid
├── quicvid/                 # main Rust app crate
│   ├── gui/                 # desktop window, controls, status display
│   ├── media/               # camera capture, encoding, decoding, rendering
│   ├── transport/           # Quinn connection, datagrams, streams, migration
│   ├── protocol/            # media/control packet formats
│   └── logging/             # frame, packet, connection, and migration logs
│
├── baseline/                # simple non-migrating video baseline
│
├── experiments/
│   ├── local/               # local two-process scripts
│   ├── two-host/            # LAN demo notes/scripts
│   ├── mininet/             # controlled migration scenarios
│   └── analysis/            # log summaries and result tables
│
├── docs/
│   ├── architecture.md
│   ├── migration-demo.md
│   ├── evaluation-plan.md
│   ├── project-history.md
│   └── limitations.md
│
└── results/
    ├── raw/
    └── summary.md
```

## Current next steps

1. Update the repository to clearly mark Quinn as the active path.
2. Mark quiche work as legacy.
3. Create a main app crate that supports `--mode server` and `--mode client`.
4. Add local two-process connection mode.
5. Add local camera preview.
6. Send test-pattern frames over Quinn datagrams.
7. Send camera frames over Quinn datagrams.
8. Display received frames in the server UI.
9. Add migration trigger during active video.
10. Add a simple baseline video transport.
11. Add comparison logs and demo scripts.
12. Add Mininet evaluation if time allows.

## Suggested final claim

A careful final claim would be:

> QuicVid demonstrates, under controlled local/LAN/Mininet conditions, that a Quinn-based video-call prototype can keep the same active media session across client-side migration events and reduce application-visible disruption compared with a simple baseline that lacks QUIC migration support.

Avoid stronger claims such as:

> QUIC guarantees seamless video calls in real-world networks.
