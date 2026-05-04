# Updated Project Plan

## 1. Project definition

QuicVid is a **Quinn-based desktop video-call prototype** designed to demonstrate robust call continuity during controlled network changes.

The engineering problem is that a video-call application is a long-lived interactive session, but the underlying transport connection often depends on the client's current network address and port. If the client changes network path, loses and regains connectivity, or receives a new NAT binding, the application may experience a frozen video, a dropped call, or a full reconnect. The user still thinks they are in the same call, but the transport may no longer recognize the peer as the same endpoint.

The concrete project question is:

> Can a desktop video-call prototype use QUIC connection migration to keep the same active media session alive across a controlled client-side address/path change, with less visible disruption than a simple baseline that lacks QUIC migration support?

The main deliverable is therefore neither a generic video app nor a standalone QUIC experiment. The final project must combine both:

> A runnable video-call prototype that sends media over Quinn/QUIC, triggers migration during an active call, preserves the same application session when migration succeeds, and compares the disruption against a simple non-migrating or reconnect-based baseline.

### 1.1 Concrete scope of achievement

The project is successful when it can demonstrate this sequence reliably:

1. start a server app instance;
2. start a client app instance;
3. establish a Quinn connection and call session;
4. send live or repeatable video frames from client to server;
5. display the received video;
6. trigger client-side migration while video is flowing;
7. show that the QUIC connection remains the same logical call when migration succeeds;
8. record frame disruption and peer address/port changes;
9. run a comparable baseline where the same disruption causes reconnect, a new session, or larger visible interruption.

### 1.2 What the project does not try to solve

The project does not aim to build a production video-conferencing system. The following are out of scope unless the core demo is already complete:

- public Internet NAT traversal;
- signaling/account infrastructure;
- mobile operating-system integration;
- multi-party conferencing;
- production-quality audio/video synchronization;
- replacing WebRTC;
- large commercial-app or user-study evaluation.

The expected final claim is narrower and more defensible: under controlled local, LAN, or Mininet conditions, QUIC migration can help a video-call prototype maintain session continuity with less application-visible disruption than a simple baseline.

---

## 2. Current reality of the repository

The old README and project plan described a much larger intended project: Qt GUI, FFmpeg, quiche, commercial video-app baselines, mobile WiFi-to-cellular handover, user studies, and statistical evaluation.

That old plan was mostly aspirational. The work that actually happened was more focused on exploring QUIC migration behavior.

The existing useful work is:

### 2.1 `quinn-ping/`

A Quinn-based QUIC ping prototype.

Existing value:

- validates basic Quinn client/server setup;
- uses QUIC streams;
- sends periodic application messages;
- exercises endpoint rebinding;
- logs latency and connection behavior;
- serves as the simplest migration sanity check.

### 2.2 `mouse-coordinates/`

A Quinn-based QUIC datagram prototype.

Existing value:

- sends real-time mouse telemetry over QUIC datagrams;
- uses a traffic pattern closer to media than ping/pong;
- shows low-latency fire-and-forget updates;
- provides a stepping stone toward video frame transport;
- demonstrates rebinding/migration behavior during active datagram traffic.

### 2.3 quiche experiments

The repository includes older C/quiche experiments and documentation.

Existing value:

- helped explore QUIC migration concepts;
- documented path validation, connection ID, and `quiche_conn_probe_path()` issues;
- is useful historical/background material.

Current decision:

- quiche is **legacy**;
- Quinn is the active implementation path;
- no new product feature should depend on quiche.

### 2.4 baseline-study documents

The old baseline-study material is useful as project motivation, but it is too large for the revised implementation plan.

Current decision:

- keep it as background;
- replace the large commercial-app/user-study plan with a small controlled baseline demo.

---

## 3. Chosen runtime architecture

The project should use a **client/server QUIC architecture**.

There will be two app instances:

1. **Server/callee instance**
   - listens on a UDP port;
   - accepts a Quinn connection;
   - receives media/control messages;
   - sends its own media back if bidirectional video is enabled;
   - logs remote address changes and connection state.

2. **Client/caller instance**
   - connects to the server;
   - initiates the call;
   - captures local camera frames;
   - sends media/control messages;
   - triggers migration in the main robustness demo.

The first product version does **not** need a signaling server, account system, peer discovery, NAT traversal, or WebRTC-style public Internet calling.

The architecture is intentionally simple:

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

For the minimum product, one-way video from client to server is acceptable during early development, but the final product should aim for bidirectional video if time allows.

---

## 4. Execution modes

The plan should support three concrete ways to run the project.

### 4.1 Mode A: Local two-process demo

This is the first required execution mode.

Both server and client run on the same development machine:

```bash
cargo run --bin quicvid -- --mode server --listen 127.0.0.1:4433
cargo run --bin quicvid -- --mode client --connect 127.0.0.1:4433
```

Purpose:

- fastest development loop;
- validates GUI, media pipeline, Quinn connection, packet format, and logging;
- can test migration through local UDP socket rebinding, usually changing local port rather than physical network.

Expected behavior:

- server window shows remote video from client;
- client window shows local preview and optionally remote video;
- migration button or shortcut triggers client-side endpoint rebinding;
- server log shows the client remote address/port change;
- QUIC connection stays associated with the same application call.

Limitations:

- does not prove real network handover;
- mostly tests port rebinding/path validation behavior;
- still valuable as the first integration demo.

### 4.2 Mode B: Two-host LAN demo

This is the main product-style demo mode.

Run the server on one host and the client on another host on the same network:

```bash
# Host A
cargo run --bin quicvid -- --mode server --listen 0.0.0.0:4433

# Host B
cargo run --bin quicvid -- --mode client --connect <server-ip>:4433
```

Purpose:

- shows the product as an actual video-call app between two machines;
- validates that media transport and GUI behavior are not only loopback artifacts;
- allows controlled disruption through interface changes, firewall rules, or socket rebinding.

Expected behavior:

- Host B sends live video to Host A;
- optionally Host A sends live video back to Host B;
- client-side migration/rebinding is triggered during the call;
- server observes remote address/port changes;
- call continues or resumes without full application restart.

Limitations:

- no NAT traversal;
- both hosts should be reachable directly;
- not a public Internet video-call product.

### 4.3 Mode C: Mininet controlled migration demo

This is the main evaluation mode.

The app runs inside a controlled Mininet topology. The exact topology can be simple at first:

```text
h1/client ─── s1 ─── h2/server
```

Later, it can include two client-side paths:

```text
             path A
h1/client ─────────── h2/server
     │                  ▲
     └──── path B ──────┘
```

Purpose:

- provides a repeatable migration/disruption scenario;
- allows scripted comparison between QUIC and baseline behavior;
- gives evidence for the final report.

Expected behavior:

- start server in the server namespace;
- start client in the client namespace;
- start video stream;
- trigger client-side migration/rebinding or path change;
- collect logs from both sides;
- summarize frame loss, freeze duration, and connection continuity.

Important note:

Mininet mode is not the first thing to build. The correct order is:

1. local two-process video call;
2. two-host video call;
3. Mininet-controlled robustness demo.

---

## 5. Application roles and responsibilities

### 5.1 Server role

The server should:

- bind to a UDP socket;
- create a Quinn endpoint;
- accept incoming QUIC connections;
- open or accept a reliable control stream;
- receive video datagrams;
- decode/render received video;
- optionally capture and send its own video;
- log connection ID, peer address, frame numbers, and migration events;
- stay alive when the client changes source address/port.

Minimum server UI:

- local status: listening / connected / in call / migrating / disconnected;
- remote video panel;
- frame rate and frame count;
- peer address display;
- migration event indicator.

### 5.2 Client role

The client should:

- connect to the server address;
- create or open a control stream;
- capture local camera frames;
- encode or packetize video;
- send video over QUIC datagrams;
- receive remote video if bidirectional mode is implemented;
- trigger migration/rebinding;
- log frame send events and migration timing.

Minimum client UI:

- connect/start button;
- stop button;
- local preview;
- remote video panel if bidirectional mode is implemented;
- migration button;
- connection state;
- migration state;
- basic statistics.

### 5.3 Shared protocol layer

The shared protocol should define:

- control messages;
- video packet format;
- optional audio packet format;
- session identifiers;
- frame sequence numbers;
- timestamps;
- keyframe markers if encoded video is used;
- experiment metadata if running in evaluation mode.

Suggested first video packet fields:

```text
message_type: video_frame_chunk
session_id
stream_id
frame_id
chunk_id
chunk_count
timestamp_ms
flags
payload
```

If frame chunks are not needed at first, the initial format can be simpler:

```text
message_type: video_frame
session_id
frame_id
timestamp_ms
payload
```

However, real QUIC datagrams have size limits, so chunking will likely become necessary once the video payload is non-trivial.

---

## 6. Transport design

### 6.1 Use QUIC streams for control

Reliable streams should carry:

- call setup;
- call accepted/rejected;
- codec/settings negotiation;
- start/stop call;
- migration debug messages;
- graceful shutdown;
- optional text/status messages.

### 6.2 Use QUIC datagrams for media

QUIC datagrams should carry:

- video frame packets/chunks;
- optional audio packets;
- low-latency telemetry.

Reason:

- media frames are time-sensitive;
- stale frames should often be dropped rather than delivered late;
- reliable streams can introduce head-of-line delay;
- the existing `mouse-coordinates` prototype already uses datagrams successfully.

### 6.3 Migration mechanism

Migration should be triggered first on the client side.

The initial migration trigger can be:

- GUI button: `Trigger migration`;
- keyboard shortcut;
- CLI flag: `--auto-migrate-after 10s`;
- experiment script command.

The first technical implementation can reuse the idea from `quinn-ping`/`mouse-coordinates`:

- rebind the Quinn endpoint to a new UDP socket/local port;
- continue sending media/control data;
- server observes a new remote address/port;
- QUIC validates the path and keeps connection state.

For stronger demos, later migration can bind to a different local interface/address where available.

---

## 7. Baseline architecture

The baseline should be simple and controlled. It does not need to be a complete alternative product.

The baseline must answer:

> What happens to the same video workload without QUIC migration support?

Recommended baseline options:

### 7.1 TCP baseline

A simple TCP video sender/receiver.

Expected behavior during address/path interruption:

- connection breaks;
- sender blocks or errors;
- app must reconnect;
- receiver sees a new session after reconnect.

This is a clear contrast to QUIC connection migration.

### 7.2 UDP session-token baseline

A simple UDP video sender/receiver with an application-level session token.

Expected behavior:

- receiver may see packets from a new address;
- continuity must be handled manually at application level;
- no transport-level migration or path validation exists;
- useful to explain what QUIC gives beyond raw datagrams.

### 7.3 Reconnect baseline

A baseline that intentionally reconnects after disruption.

Expected behavior:

- visible pause;
- new connection/session;
- application-level recovery required.

Minimum requirement:

- implement at least one baseline;
- the baseline must use the same or similar video workload;
- the final demo must compare disruption against QuicVid.

---

## 8. Definition of done

The project is done when all P0 items are complete.

### 8.1 Product-level done

- QuicVid runs as a desktop app.
- The app can run in server mode.
- The app can run in client mode.
- The server and client can run as separate processes.
- The server and client can run on the same host for local testing.
- The server and client can run on two reachable hosts for product-style testing.
- The client captures live camera video.
- The receiver displays remote video.
- Video data is transported over Quinn/QUIC.
- The UI shows call state and connection state.
- The app can start and stop a call cleanly.

### 8.2 Migration-level done

- Migration can be triggered during an active video call.
- Migration does not silently restart the whole application session.
- The app logs migration start time.
- The app logs whether the QUIC connection survived.
- The server logs peer address/port changes.
- The app measures or estimates frame disruption around migration.
- The demo shows that the QUIC call continues or resumes with bounded disruption.

### 8.3 Robustness-demo done

- A baseline video transport exists.
- The same style of disruption/migration is applied to the baseline.
- The baseline shows larger disruption, reconnect, drop, or new-session behavior.
- The QUIC demo and baseline demo can be run from documented commands.
- A short summary compares the two.

### 8.4 Documentation-level done

- README describes the actual Quinn-based direction.
- README explains local two-process mode.
- README explains two-host mode.
- README explains Mininet/evaluation mode if implemented.
- README marks quiche work as legacy.
- docs explain architecture, migration behavior, evaluation setup, and limitations.
- final report can be written from repo documentation and results.

---

## 9. Minimum successful demo

The minimum successful demo has two parts.

### 9.1 Baseline demo

Run:

```bash
cargo run --bin quicvid-baseline -- --mode server --listen 0.0.0.0:5000
cargo run --bin quicvid-baseline -- --mode client --connect <server-ip>:5000
```

Then trigger disruption:

```bash
# Example only; exact command depends on final setup
./experiments/trigger-baseline-disruption.sh
```

Show:

- video freezes or stops;
- reconnect is required or a new session starts;
- logs record the interruption.

### 9.2 QUIC demo

Run:

```bash
cargo run --bin quicvid -- --mode server --listen 0.0.0.0:4433
cargo run --bin quicvid -- --mode client --connect <server-ip>:4433
```

Then trigger migration:

```bash
# Example options
press "M" in the GUI
# or
cargo run --bin quicvid -- --mode client --connect <server-ip>:4433 --auto-migrate-after 10s
```

Show:

- video continues or resumes without full application reconnect;
- same call/session remains active;
- server observes new peer address/port;
- logs summarize frame disruption.

Example summary:

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

---

## 10. Milestones as vertical slices

Each milestone should produce a runnable or inspectable result. Avoid long horizontal work where media, GUI, and transport are built separately for weeks without integration.

### Milestone 1 — Reset the repository around the Quinn path

Done when the repository clearly says what is active and what is legacy.

GitHub issue titles:

- Update README to describe the Quinn-based client/server architecture
- Document the decision to move from quiche to Quinn
- Mark quiche experiments as legacy background work
- Move old aspirational plans into project-history documentation
- Verify `quinn-ping` still builds and runs
- Verify `mouse-coordinates` still builds and runs
- Document how existing Quinn demos are run today
- Add a current-status document for existing prototypes
- Remove quiche from the active implementation roadmap
- Add a project glossary for client, server, migration, rebinding, and baseline

### Milestone 2 — Build the minimal client/server app shell

Done when two QuicVid app instances can run as server and client and show connection state.

GitHub issue titles:

- Create the main `quicvid` Rust application crate
- Add `--mode server` and `--mode client` command-line options
- Add `--listen` option for server mode
- Add `--connect` option for client mode
- Add a minimal desktop window for the server role
- Add a minimal desktop window for the client role
- Add Start Call and Stop Call controls
- Add server listening state to the UI
- Add client connecting state to the UI
- Add connected and disconnected states to the UI
- Run the Quinn server task from the app shell
- Run the Quinn client task from the app shell
- Add channel-based communication between GUI and transport tasks
- Add local two-process demo instructions
- Add structured logs for app startup and connection state

### Milestone 3 — Add local video capture inside the app

Done when at least the client app can show local camera preview.

GitHub issue titles:

- Add camera device discovery
- Add camera capture task
- Display local camera preview in the client UI
- Display local camera preview in the server UI if bidirectional mode is enabled
- Add fallback test-pattern video source
- Add camera start and stop lifecycle handling
- Add frame timestamp metadata
- Add frame sequence numbers before network transport
- Add local frame-rate counter
- Add configurable camera resolution
- Add configurable camera frame rate
- Add camera unavailable error message
- Document supported local video sources

### Milestone 4 — Send the first video frames over Quinn datagrams

Done when one-way live video works from client to server over QUIC.

GitHub issue titles:

- Define the first video datagram packet format
- Add media session ID to video packets
- Send test-pattern frames over QUIC datagrams
- Receive test-pattern frames over QUIC datagrams
- Render received test-pattern frames in the server UI
- Send camera frames over QUIC datagrams
- Render received camera frames in the server UI
- Add frame sequence number validation on receive
- Add frame receive timestamp logging
- Add frame send timestamp logging
- Drop stale video frames on the receiver
- Add received-frame-rate display
- Add sent-frame-rate display
- Add frame loss estimate for one-way video
- Add two-process local video-over-QUIC demo script

### Milestone 5 — Make video practical enough for the robustness demo

Done when the video stream is small and stable enough to survive normal local/two-host testing.

GitHub issue titles:

- Decide whether first demo uses raw frames, JPEG frames, or encoded video
- Add frame compression for the first practical video demo
- Add video frame decompression on receive
- Add datagram chunking for oversized video frames
- Add frame reassembly from chunks
- Drop incomplete stale frames after timeout
- Add low-resolution demo preset
- Add low-frame-rate demo preset
- Add configurable media quality preset
- Add keyframe marker if encoded video requires it
- Add decoder error logging
- Add video backlog monitoring
- Add receiver-side frame dropping under backlog
- Document media quality trade-offs for the demo

### Milestone 6 — Make the app run across two hosts

Done when the same app works on two reachable machines.

GitHub issue titles:

- Add server bind address documentation for LAN testing
- Add client server-address configuration documentation
- Add certificate/trust handling for two-host Quinn testing
- Add firewall troubleshooting notes for UDP server port
- Verify one-way video over QUIC between two hosts
- Verify optional bidirectional video between two hosts
- Add two-host demo checklist
- Add network address display to the UI
- Log local and remote socket addresses
- Add connection failure diagnostics for two-host mode

### Milestone 7 — Add migration to the active video call

Done when migration can be triggered while the video stream is active.

GitHub issue titles:

- Add manual migration trigger in client mode
- Add `--auto-migrate-after` option for client mode
- Rebind the Quinn client endpoint during an active video call
- Continue sending video after endpoint rebinding
- Show migrating state in the client UI
- Show migration observed state in the server UI
- Log migration start timestamp on the client
- Log post-migration peer address on the server
- Log stable QUIC connection identity during migration
- Measure frames sent during the migration window
- Measure frames received during the migration window
- Estimate visible video freeze duration around migration
- Add migration failure state to the UI
- Add migration demo instructions for local two-process mode
- Add migration demo instructions for two-host mode

### Milestone 8 — Add the baseline comparison

Done when the QUIC benefit can be demonstrated against a simple baseline.

GitHub issue titles:

- Define the baseline transport choice for the final demo
- Implement a TCP video baseline using the same video source
- Add baseline server mode
- Add baseline client mode
- Add baseline frame send logging
- Add baseline frame receive logging
- Add baseline session ID logging
- Add baseline reconnect behavior after disruption
- Add baseline disruption trigger script
- Add QUIC migration trigger script for comparison
- Add shared demo workload for QUIC and baseline runs
- Add summary output comparing QUIC and baseline disruption
- Add side-by-side demo instructions
- Document what the baseline comparison proves and does not prove

### Milestone 9 — Add controlled Mininet evaluation

Done when the robustness demo can be repeated in a controlled environment.

GitHub issue titles:

- Add minimal Mininet topology for QuicVid client and server
- Add Mininet script to start the QUIC server
- Add Mininet script to start the QUIC client
- Add Mininet script to start the baseline server
- Add Mininet script to start the baseline client
- Add scripted client-side migration event
- Add scripted baseline disruption event
- Save client and server logs from Mininet runs
- Add experiment ID and trial ID to all logs
- Add repeated QUIC migration trial script
- Add repeated baseline trial script
- Add log parser for frame loss around migration
- Add log parser for visible freeze estimate
- Add result summary table for Mininet trials
- Document Mininet setup and limitations

### Milestone 10 — Add audio if video and migration are stable

Done when basic audio works, or it is clearly marked as experimental and not required for core success.

GitHub issue titles:

- Add microphone device discovery
- Add microphone capture task
- Define audio datagram packet format
- Send audio packets over QUIC datagrams
- Receive audio packets over QUIC datagrams
- Add basic audio playback
- Add mute and unmute control
- Add audio packet sequence numbers
- Add small audio jitter buffer
- Log audio underflow and overflow events
- Add option to run in video-only mode
- Document audio support status

### Milestone 11 — Product hardening and final documentation

Done when another person can run the demos from the repository.

GitHub issue titles:

- Add one-command local QUIC demo script
- Add one-command two-host QUIC demo checklist
- Add one-command baseline demo script
- Add one-command robustness comparison script
- Add clear error messages for failed connection setup
- Add clear error messages for camera failure
- Add clean call teardown
- Add clean media task shutdown
- Add clean transport task shutdown
- Add troubleshooting section to README
- Add final architecture diagram
- Add migration sequence diagram
- Add final evaluation results summary
- Add limitations document
- Add future work document
- Add final report outline
- Add final presentation outline
- Tag final project release

---

## 11. Priority guide

### P0 — Required for project success

- Quinn is the active implementation path.
- App supports server mode and client mode.
- Server and client run as separate processes.
- Local two-process demo works.
- Two-host demo works or has a clear documented blocker.
- Client captures live video.
- Receiver displays remote video.
- Video is transported over Quinn.
- Migration can be triggered during active video.
- Baseline contrast exists.
- Logs summarize QUIC vs baseline behavior.
- README explains how to run the demo.

### P1 — Important for a strong project

- Video compression/chunking.
- Bidirectional video.
- Mininet repeated trials.
- Audio.
- Result tables.
- UI polish.
- Better failure handling.

### P2 — Nice to have

- Commercial app comparison.
- User study.
- Advanced charts.
- Public Internet support.
- NAT traversal.
- Mobile handover.
- Multi-party calls.

---

## 12. Suggested four-week schedule

This assumes roughly 180-200 hours total.

### Week 1 — Quinn path, app shell, local video

Target outcome:

- repo reflects Quinn direction;
- app can run as server or client;
- local video preview works;
- local two-process connection works.

Main work:

- Milestone 1;
- Milestone 2;
- first half of Milestone 3.

### Week 2 — Video over QUIC and two-host demo

Target outcome:

- client sends video over QUIC;
- server displays remote video;
- two-host mode works if network setup allows;
- logs show frame send/receive counts.

Main work:

- finish Milestone 3;
- Milestone 4;
- parts of Milestone 5;
- Milestone 6.

### Week 3 — Migration and baseline comparison

Target outcome:

- migration happens during active video;
- QUIC session continuity is logged;
- baseline demo exists;
- the benefit of QUIC becomes visible.

Main work:

- Milestone 7;
- Milestone 8.

### Week 4 — Controlled evaluation, hardening, docs

Target outcome:

- repeated demo or Mininet trials exist;
- results are summarized;
- README and docs are final enough;
- project can be presented and defended.

Main work:

- Milestone 9 if feasible;
- Milestone 10 only if core video/migration is stable;
- Milestone 11.

---

## 13. Final claim to aim for

A careful final claim:

> QuicVid demonstrates that a Quinn-based video-call prototype can preserve an active media session across controlled client-side migration events, reducing application-visible disruption compared with a simple baseline that lacks QUIC migration support.

Avoid claiming:

> QUIC guarantees seamless video calls in real-world networks.

The project should be honest: it demonstrates a controlled product-level benefit, not a production replacement for WebRTC.
