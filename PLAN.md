# QuicVid Project Plan

## Week-by-week epic assignment

This table reflects the current four-week execution focus. Since Week 1 was mostly spent on Epic 1.1, the heavier implementation work is shifted into Weeks 2-4. The weeks are planning buckets rather than strict deadlines: unfinished P0 work should carry forward before starting lower-priority work.

| Week | Main focus | Epics | Target outcome |
|---|---|---|---|
| Week 1 | Project recovery and scope alignment | Epic 1.1 — Recover project state and align QuicVid around Quinn | The repository direction is clarified: Quinn is the active path, quiche is legacy/background, current prototypes are documented, and the plan is aligned around a fake-video-first baseline. |
| Week 2 | Main app skeleton and first fake-video stream | Epic 1.2 — Build the initial QuicVid client/server app skeleton<br>Start Epic 1.3 — Stream fake video frames over Quinn datagrams | The new `quic-vid` app runs as client/server, exchanges a control message, writes structured logs, and begins sending numbered fake frames over QUIC datagrams. |
| Week 3 | Complete fake-video baseline and manual migration | Finish Epic 1.3 — Stream fake video frames over Quinn datagrams<br>Epic 2.1 — Demonstrate manual QUIC migration during fake video streaming | The app has a measurable fake-video workload with frame IDs, receive gaps, missing/out-of-order detection, and a manual migration demo during active fake-video streaming. |
| Week 4 | Migration controller, automatic strategy, and evidence packaging | Epic 2.2 — Add migration controller subsystem<br>Start/simplify Epic 2.3 — Implement automatic migration strategy for fake video<br>Epic 4.1/4.3 essentials — logging analysis and final demo docs | Migration is routed through a clear controller, a simple automatic trigger works if time allows, core logs/results are summarized, and the final demo/release path is documented. |

Stretch work after the P0 path is stable:

```text
Epic 3.4 — baseline transport comparison
Epic 3.1/3.2/3.3 — real-video path
Epic 4.2 — physical WiFi-to-WiFi demo
Optional — bidirectional video and audio
```

Priority rule:

```text
P0 fake-video migration evidence first
P1 real-video, baseline comparison, and physical WiFi demo second
P2 bidirectional video and audio only as future work
```

## 1. Project definition

QuicVid is a **Quinn-based video-call prototype** for exploring how QUIC connection migration can preserve a long-lived real-time media session across controlled network changes.

The engineering problem is that a video call is perceived by the user as one continuous session, but the transport path underneath it may change. When a device switches WiFi networks, changes interface, receives a new IP address, or gets a new UDP port/NAT binding, a traditional transport session may break or require the application to reconnect. QuicVid investigates whether QUIC migration can reduce that disruption while preserving the same logical call session.

The concrete project question is:

> Can a Quinn-based video-call prototype preserve the same active media session across controlled client-side address or path changes, with less application-visible disruption than a simple reconnect-style baseline?

The project is not a generic video app and not only a QUIC experiment. The intended final artifact is:

> A runnable QuicVid prototype that sends media-like traffic over Quinn/QUIC, triggers migration during an active session, records disruption, and compares QUIC migration against a simple baseline using the same workload.

## 2. Concrete scope of achievement

The project should aim to demonstrate this sequence:

1. start a server instance;
2. start a client instance;
3. establish a Quinn connection and logical call session;
4. send repeatable fake video frames over QUIC datagrams;
5. measure frame delivery with frame IDs, receive gaps, missing frames, and out-of-order frames;
6. trigger migration during active fake-video streaming;
7. preserve the same logical session when migration succeeds;
8. add an automatic migration decision based on media degradation;
9. compare the result against a simple baseline using the same workload;
10. optionally add real visual video and a physical WiFi-to-WiFi demo if the core path is stable.

The most important design decision is:

> Build and evaluate fake numbered video frames first. Add real video only after the fake-video migration path is working.

Fake frames make the project measurable earlier. They also avoid letting camera APIs, GUI rendering, compression, and frame chunking block the central QUIC migration result.

## 3. Non-goals

QuicVid does not aim to build a production video-conferencing system.

Out of scope unless the core project is already complete:

- public Internet NAT traversal;
- user accounts or signaling infrastructure;
- mobile OS integration;
- multi-party conferencing;
- production-grade audio/video synchronization;
- replacing WebRTC;
- commercial video-app benchmarking;
- user studies;
- guaranteed seamless migration in arbitrary real-world networks.

A careful final claim should be:

> QuicVid demonstrates, under controlled conditions, that a Quinn-based video-call prototype can preserve an active media-like session across selected client-side migration events and reduce application-visible disruption compared with a simple baseline that lacks QUIC migration support.

## 4. Current repository state

The project previously had a broader and more aspirational scope, including quiche, Qt/GUI work, FFmpeg, commercial-app baselines, mobile handover, and larger statistical evaluation. The current plan narrows the project into a product-shaped Quinn prototype with a staged fake-video-first migration path.

### 4.1 Active path: Quinn

New product work should use **Quinn** as the active QUIC implementation path.

Existing Quinn-related prototypes:

- `quinn-ping/`: basic Quinn client/server and migration/rebinding reference;
- `mouse-coordinates/`: QUIC datagram telemetry prototype and useful reference for fake-video datagrams.

### 4.2 Legacy/background path: quiche

Older C/quiche work is retained as background material only.

It helped explore:

- QUIC connection IDs;
- path validation;
- connection migration behavior;
- low-level migration API issues.

However, no new product feature should depend on quiche unless the project scope is explicitly changed.

### 4.3 Baseline-study documents

The old baseline-study material is useful as project motivation, but the revised project should use a smaller controlled baseline comparison. The baseline must reuse the same fake/real video workload, frame IDs, logging schema, and disruption metrics as QuicVid where possible.

## 5. Runtime architecture

QuicVid should use a simple client/server architecture.

```text
Client instance                            Server instance
---------------                            ---------------
CLI / optional GUI                         CLI / optional GUI
Fake video source                          Frame receiver
Camera/test-pattern source later           Remote preview later
Control stream sender  ─── QUIC stream ─▶  Control stream receiver
Media datagram sender ─── QUIC datagrams ▶ Media datagram receiver
Migration controller                       Migration observer
JSONL logs                                 JSONL logs
```

### 5.1 Server responsibilities

The server should:

- bind to a UDP socket;
- create a Quinn endpoint;
- accept incoming QUIC connections;
- receive control messages over streams;
- receive fake or real video frames over datagrams;
- track frame IDs, missing frames, out-of-order frames, and receive gaps;
- observe peer address/port changes where visible;
- write structured JSONL logs;
- print run summaries.

### 5.2 Client responsibilities

The client should:

- connect to the server;
- generate a session ID;
- send an initial client hello over a QUIC stream;
- generate fake video frames first;
- send fake or real frames over QUIC datagrams;
- monitor media/network quality where needed;
- trigger manual or automatic migration;
- optionally bind to a selected local address or interface;
- write structured JSONL logs;
- print run summaries.

### 5.3 Control streams

QUIC streams should carry reliable control messages such as:

- client hello;
- session setup;
- selected mode and parameters;
- migration debug messages;
- graceful shutdown.

### 5.4 Media datagrams

QUIC datagrams should carry time-sensitive media-like payloads:

- fake video frames in the early milestones;
- test-pattern or camera frames later;
- optional audio only if core video/migration work is stable.

Datagrams are appropriate because old media frames are often less useful than fresh ones. Late frames should be measurable and droppable rather than blocking the stream.

## 6. Execution modes

QuicVid should support four execution modes over the course of the project.

### 6.1 Local two-process mode

Client and server run on the same development machine.

Purpose:

- fastest development loop;
- first app integration target;
- local QUIC connection and fake-video tests;
- local migration/rebinding sanity checks.

### 6.2 Two-device LAN mode

Server runs on one physical device and client runs on another reachable device.

Purpose:

- proves real client/server operation;
- supports product-style demonstrations;
- prepares for physical WiFi switching.

### 6.3 Mininet mode

Client and server run in a controlled network topology.

Purpose:

- repeatable migration/disruption experiments;
- structured comparison between QUIC and baseline;
- main evidence path for the final report.

### 6.4 Physical WiFi-to-WiFi migration mode

Server remains reachable while the client switches between WiFi networks.

Purpose:

- persuasive product demo;
- shows the idea in a realistic setting;
- high value but risky due to routing, firewall, OS, and network behavior.

Mininet should be treated as the controlled evidence path. Physical WiFi should be treated as a strong demo path, not the only proof of success.

## 7. Migration controller design

Migration should not be implemented as random rebinding calls scattered through the code. QuicVid should have a dedicated migration controller.

The migration controller should own:

- migration mode: `off`, `manual`, `automatic`;
- migration state: `healthy`, `degraded`, `migration_candidate`, `migrating`, `recovered`, `failed`;
- configured thresholds;
- cooldown behavior;
- selected local bind address or interface if supported;
- migration start/completion/failure events;
- disruption summary around migration.

Suggested first automatic policy:

```text
healthy
  -> degraded if receive gap exceeds warning threshold
  -> migration_candidate if receive gap exceeds migration threshold
  -> migrating when rebind/migration is triggered
  -> recovered if frames resume within expected window
  -> failed if frames do not resume
```

A minimal first automatic trigger is enough:

```text
receive gap > threshold -> trigger migration -> enter cooldown
```

Missing-frame thresholds can be added later.

## 8. Baseline design

The baseline must use the same workload as QuicVid. It should not be a separate toy that sends unrelated data.

The baseline should reuse:

- fake video frame source;
- real/test-pattern video source if implemented;
- frame IDs;
- session IDs where useful;
- JSONL logging schema;
- frame loss and receive-gap metrics;
- summary scripts.

Recommended baseline choices:

1. **TCP/reconnect baseline**: the connection breaks or reconnects after disruption, causing a visible/session-level interruption.
2. **Non-migrating UDP baseline**: demonstrates what has to be handled manually without transport-level migration.

A simple reconnect baseline is likely the clearest first comparison.

## 9. Milestone structure

The project should be organized into four major milestones. Each milestone has epic issues, and each epic has vertical-slice task issues.

```text
Milestone 1 — Recover and build the fake-video QUIC baseline
Milestone 2 — Manual and automatic QUIC migration with fake video
Milestone 3 — Real video and QUIC vs baseline comparison
Milestone 4 — Evaluation, physical demo, and final release
```

This replaces the older 11-milestone structure. The new structure keeps the project board readable while preserving the stronger fake-video-first design.

---

# Milestone 1 — Recover and build the fake-video QUIC baseline

## Goal

Restart the project cleanly, align it around Quinn, and build the first product-shaped QuicVid application that streams numbered fake video frames over QUIC datagrams.

This milestone answers:

> Can we run separate client/server QuicVid instances and stream measurable video-like frames over Quinn?

## Deliverable

A Quinn-based QuicVid client/server app that:

- runs in server and client modes;
- exchanges an initial control message over a QUIC stream;
- generates a session ID;
- sends numbered fake video frames over QUIC datagrams;
- logs frame-level delivery behavior;
- detects missing and out-of-order frames;
- prints frame stream summaries.

## Epic 1.1 — Recover project state and align QuicVid around Quinn

Purpose: clarify active scope, document existing prototypes, and mark old work as legacy/background.

Sub-issues:

- Add `PLAN.md` with revised Quinn-based product scope
- Update `README.md` with engineering problem and concrete achievement scope
- Document September project history and scope reset
- Document Quinn as the active QUIC implementation path
- Mark quiche experiments and notes as legacy
- Verify `quinn-ping` still builds
- Verify `quinn-ping` still runs as client and server
- Verify `mouse-coordinates` still builds
- Verify `mouse-coordinates` still sends QUIC datagrams
- Document current runnable demo commands
- Add `docs/current-status.md`
- Add local development setup notes
- Clean obsolete generated files and macOS metadata

Estimate: **12–20 h**

## Epic 1.2 — Build the initial QuicVid client/server app skeleton

Purpose: create the first product-shaped app foundation.

Sub-issues:

- Create the main QuicVid app crate
- Add CLI mode selection for server and client
- Add server listen address configuration
- Add client connect address configuration
- Add local bind address configuration
- Add migration mode CLI option
- Copy minimal Quinn server setup into the app
- Copy minimal Quinn client setup into the app
- Send initial client hello over a QUIC stream
- Receive and log client hello on the server
- Add session ID generation for client runs
- Add structured JSONL event logging
- Add graceful shutdown handling
- Add local two-process run instructions
- Add one-command local smoke test script

Estimate: **25–40 h**

Recommended naming:

```text
Product name: QuicVid
Cargo package: quic-vid
Rust crate: quic_vid
CLI binary: quic-vid
```

## Epic 1.3 — Stream fake video frames over Quinn datagrams

Purpose: create the first media-shaped workload before real video complexity.

Sub-issues:

- Define the first QuicVid datagram frame format
- Add fake video frame generator
- Send numbered fake video frames from client to server
- Receive fake video frames on the server
- Log fake frame send events
- Log fake frame receive events
- Detect missing fake frame IDs
- Detect out-of-order fake frame IDs
- Add configurable fake video frame rate
- Add configurable fake video run duration
- Add configurable fake frame payload size
- Add frame stream summary at shutdown
- Add fake-video local demo command
- Document fake video datagram mode
- Add basic tests for fake frame format and frame tracking

Estimate: **30–45 h**

## Milestone 1 estimate

Total: **67–105 h**

---

# Milestone 2 — Manual and automatic QUIC migration with fake video

## Goal

Use the fake-video workload to demonstrate the core migration idea: the same logical session can continue through controlled QUIC migration, and the application can decide when to migrate based on media degradation.

This is the technical heart of the project.

## Deliverable

A fake-video QUIC demo where migration can be triggered during active datagram streaming, disruption is measured, and automatic migration can be triggered from media-quality degradation.

## Epic 2.1 — Demonstrate manual QUIC migration during fake video streaming

Sub-issues:

- Add manual migration trigger for the client
- Add periodic migration trigger for fake video mode
- Reuse endpoint rebinding ideas from `quinn-ping`
- Log migration start on the client
- Log migration completion on the client
- Log server-observed peer address changes
- Log stable call/session identity across migration
- Measure fake frame loss around migration
- Measure fake frame receive gap around migration
- Print migration disruption summary at shutdown
- Add local fake-video migration demo command
- Add Mininet fake-video migration scenario
- Document how to reproduce fake-video migration

Estimate: **30–50 h**

## Epic 2.2 — Add migration controller subsystem

Sub-issues:

- Define migration controller responsibilities
- Add migration mode: `off`, `manual`, `automatic`
- Add migration state: `healthy`, `degraded`, `migrating`, `recovered`, `failed`
- Add migration event model
- Add migration cooldown configuration
- Add selected local bind address or interface field
- Add migration result tracking
- Route manual migration trigger through migration controller
- Route periodic migration trigger through migration controller
- Log migration state transitions
- Document migration controller design

Estimate: **20–35 h**

## Epic 2.3 — Implement automatic migration strategy for fake video

Sub-issues:

- Define automatic migration strategy for QuicVid
- Add media quality monitor for received video frames
- Track latest received frame timestamp
- Track consecutive missing video frame IDs
- Track largest receive gap during a call
- Add healthy connection state
- Add degraded connection state
- Add migration-candidate connection state
- Trigger migration after video receive gap threshold
- Trigger migration after consecutive missing frame threshold
- Add migration cooldown to prevent repeated rebinding loops
- Log automatic migration trigger reason
- Log pre-migration media quality snapshot
- Log post-migration media recovery time
- Expose automatic migration mode in the CLI
- Add configurable migration thresholds
- Add controlled degradation scenario for automatic migration
- Add fake-video automatic migration demo
- Compare manual migration and automatic migration behavior
- Document the automatic migration strategy
- Document limitations of the migration strategy

Estimate: **40–70 h**

## Milestone 2 estimate

Total: **90–155 h**

Fallback if time is tight:

```text
receive gap > threshold -> trigger migration -> cooldown
```

The missing-frame trigger can be added later.

---

# Milestone 3 — Real video and QUIC vs baseline comparison

## Goal

Turn the fake-video migration result into a more visible video prototype and compare QUIC migration against a simple baseline using the same workload.

Real video is valuable for demo quality, but the project should not depend entirely on camera/GUI success. The minimum version can use test-pattern visual frames.

## Deliverable

A visible one-way video or test-pattern stream over Quinn, plus a baseline comparison that demonstrates why QUIC migration helps.

## Epic 3.1 — Add local video capture and preview

Sub-issues:

- Choose the initial Rust-native GUI framework
- Add minimal GUI window
- Add app state display to the GUI
- Add local video preview panel
- Add camera device discovery
- Add camera frame capture
- Display captured camera frames in the local preview
- Add fallback test-pattern video source
- Add camera unavailable error message
- Add configurable camera resolution
- Add configurable camera frame rate
- Add local preview frame-rate display
- Add start and stop preview controls
- Document local preview setup

Estimate: **35–60 h**

## Epic 3.2 — Stream real video over Quinn from client to server

Sub-issues:

- Define the first real video frame payload format
- Convert captured frames into transport payloads
- Add frame chunking when payload exceeds datagram size
- Send local video frames over Quinn datagrams
- Receive video frame datagrams on the server
- Reconstruct received video frames
- Display received video frames in a remote video panel
- Drop stale video frames under backlog
- Log real video frame send events
- Log real video frame receive events
- Log video decode or reconstruction failures
- Add remote video frame-rate display
- Add video-only client-to-server demo command
- Document the one-way real video demo

Estimate: **45–80 h**

## Epic 3.3 — Demonstrate robust real-video continuity with QUIC migration

Sub-issues:

- Enable manual migration trigger during real video streaming
- Enable automatic migration trigger during real video streaming
- Show migration state in the client UI
- Show migration state in the server UI
- Log video migration start event
- Log video migration completion event
- Log server-observed address change during video call
- Measure frame loss during real video migration
- Measure visible receive gap during real video migration
- Show post-migration video recovery in the UI
- Add real-video migration summary output
- Add one-command real-video migration demo
- Add Mininet real-video migration scenario
- Document the QUIC real-video migration demo

Estimate: **35–60 h**

## Epic 3.4 — Add baseline video transport to show the benefit of QUIC

Sub-issues:

- Choose the minimal baseline transport design
- Add baseline mode to the app CLI
- Reuse fake video workload in baseline mode
- Reuse real video workload in baseline mode
- Log baseline session start events
- Log baseline session restart events
- Trigger comparable interruption scenario for baseline mode
- Measure baseline frame loss around interruption
- Measure baseline visible receive gap
- Add reconnect behavior for baseline mode
- Add baseline summary output
- Add side-by-side QUIC versus baseline demo script
- Document the baseline comparison scenario
- Document what the baseline comparison proves
- Document what the baseline comparison does not prove

Estimate: **35–60 h**

## Milestone 3 estimate

Total: **150–260 h**

Minimum acceptable Milestone 3 if time is tight:

- test-pattern visual frames instead of full camera support;
- one-way video only;
- simple reconnect baseline;
- same fake-video logging and metrics reused.

---

# Milestone 4 — Evaluation, physical demo, and final release

## Goal

Turn the prototype into presentable evidence: structured logs, analysis scripts, demo instructions, and optionally a physical WiFi-to-WiFi migration demo.

## Deliverable

A final release with runnable demos, summarized results, limitations, and clear instructions.

## Epic 4.1 — Add experiment logging and analysis for migration robustness

Sub-issues:

- Define stable JSONL experiment log schema
- Add run ID to all log events
- Add scenario name to all log events
- Add migration window markers to logs
- Add frame disruption summary script
- Add receive-gap summary script
- Add migration recovery-time summary script
- Add QUIC versus baseline comparison script
- Add CSV export for summarized results
- Add markdown result table generation
- Add sample result logs from local demo
- Add sample result logs from Mininet demo
- Document experiment log format
- Document how to interpret migration results

Estimate: **35–60 h**

## Epic 4.2 — Demonstrate physical WiFi-to-WiFi migration with QuicVid

Priority: **P1 / stretch-core**

This is valuable, but the project should remain successful if the physical demo is only partially achieved. Mininet should remain the controlled evidence path.

Sub-issues:

- Add physical WiFi migration demo plan
- Document required network topology for WiFi-to-WiFi migration
- Add server mode suitable for stable LAN host
- Add client mode suitable for WiFi switching
- Add explicit local bind address option for client
- Add network interface selection option for client
- Add manual rebind command for physical WiFi demo
- Add automatic rebind after receive-gap detection
- Log local client socket address before WiFi switch
- Log local client socket address after WiFi switch
- Log server-observed peer address before WiFi switch
- Log server-observed peer address after WiFi switch
- Log call/session ID across WiFi switch
- Measure frame gap during physical WiFi switch
- Measure recovery time after physical WiFi switch
- Add two-device fake-video WiFi-switch demo
- Add two-device real-video WiFi-switch demo
- Add checklist for server firewall and port access
- Add troubleshooting notes for unreachable server after SSID switch
- Add baseline WiFi-switch comparison
- Document limitations of physical WiFi migration demo

Estimate: **50–90 h**

## Epic 4.3 — Package final QuicVid demo, documentation, and release

Sub-issues:

- Write final setup instructions
- Write local two-process demo instructions
- Write two-host LAN demo instructions
- Write Mininet demo instructions
- Write physical WiFi-to-WiFi demo instructions
- Write QUIC versus baseline demo instructions
- Add architecture diagram
- Add protocol diagram
- Add migration sequence diagram
- Add automatic migration strategy diagram
- Add final results summary
- Add final limitations section
- Add future work section
- Add troubleshooting section
- Add screenshots or demo images
- Clean unused legacy files from active paths
- Verify fresh-clone setup
- Tag final project release

Estimate: **35–60 h**

## Milestone 4 estimate

Total: **120–210 h**

Required part without physical WiFi stretch: **70–120 h**

---

# 10. Final recommended epic hierarchy

```text
Milestone 1 — Recover and build the fake-video QUIC baseline
├── Epic 1.1: Recover project state and align QuicVid around Quinn
├── Epic 1.2: Build the initial QuicVid client/server app skeleton
└── Epic 1.3: Stream fake video frames over Quinn datagrams

Milestone 2 — Manual and automatic QUIC migration with fake video
├── Epic 2.1: Demonstrate manual QUIC migration during fake video streaming
├── Epic 2.2: Add migration controller subsystem
└── Epic 2.3: Implement automatic migration strategy for fake video

Milestone 3 — Real video and QUIC vs baseline comparison
├── Epic 3.1: Add local video capture and preview
├── Epic 3.2: Stream real video over Quinn from client to server
├── Epic 3.3: Demonstrate robust real-video continuity with QUIC migration
└── Epic 3.4: Add baseline video transport to show the benefit of QUIC

Milestone 4 — Evaluation, physical demo, and final release
├── Epic 4.1: Add experiment logging and analysis for migration robustness
├── Epic 4.2: Demonstrate physical WiFi-to-WiFi migration with QuicVid
└── Epic 4.3: Package final QuicVid demo, documentation, and release
```

---

# 11. Priority guide

## P0 — Required for a respectable project

- Epic 1.1: Recover project state and align QuicVid around Quinn
- Epic 1.2: Build the initial QuicVid client/server app skeleton
- Epic 1.3: Stream fake video frames over Quinn datagrams
- Epic 2.1: Demonstrate manual QUIC migration during fake video streaming
- Epic 2.2: Add migration controller subsystem
- Epic 2.3: Implement automatic migration strategy for fake video
- Epic 3.4: Add baseline video transport to show the benefit of QUIC
- Epic 4.1: Add experiment logging and analysis for migration robustness
- Epic 4.3: Package final QuicVid demo, documentation, and release

## P1 — Strong product/demo value

- Epic 3.1: Add local video capture and preview
- Epic 3.2: Stream real video over Quinn from client to server
- Epic 3.3: Demonstrate robust real-video continuity with QUIC migration
- Epic 4.2: Demonstrate physical WiFi-to-WiFi migration with QuicVid

## P2 — Optional product completeness

- Extend QuicVid from one-way video to bidirectional calling
- Add experimental audio support

Real video is valuable, but the project should not depend completely on it. The strongest defensible minimum is fake-video migration, automatic migration strategy, baseline comparison, logging, and analysis.

---

# 12. Suggested time-boxed implementation path

The implementation order should follow vertical slices, not a strict epic-number sequence.

Recommended order:

1. recover docs and existing demos;
2. build client/server skeleton;
3. send fake video over Quinn;
4. add manual migration for fake video;
5. add migration controller;
6. add automatic migration for fake video;
7. add baseline comparison using fake video;
8. add logging/analysis scripts;
9. add real video or test-pattern visuals;
10. try physical WiFi-to-WiFi demo;
11. package final demo and documentation.

If time becomes tight, prioritize:

```text
fake-video stream -> manual migration -> automatic migration -> baseline -> logs/results -> final docs
```

and leave real video, GUI polish, physical WiFi, bidirectional calling, and audio as stretch/future work.

---

# 13. Final claim to aim for

A careful final claim:

> QuicVid demonstrates that a Quinn-based video-call prototype can preserve an active media-like session across controlled client-side migration events, reducing application-visible disruption compared with a simple baseline that lacks QUIC migration support.

Avoid claiming:

> QUIC guarantees seamless video calls in real-world networks.

The project should be honest: it demonstrates a controlled product-level benefit, not a production replacement for WebRTC.
