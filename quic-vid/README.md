# QuicVid

QuicVid is a Quinn-based prototype for experimenting with QUIC media transport
and connection migration.

The current application provides:

* separate client and server modes;
* configurable server and client addresses;
* configurable client local bind address;
* a logical QuicVid session UUID;
* control-stream session setup and end-of-run coordination;
* a reusable chunk-aware media datagram format;
* generated fake-video traffic over QUIC DATAGRAMs;
* configurable frame rate, run duration, and payload size;
* detection of missing, duplicate, and out-of-order frames;
* receive-gap measurement and end-of-run delivery summaries;
* connection, session, and address logging.

## Run locally

Start the server:

```bash
cargo run -- server --listen 127.0.0.1:4433
```

Start the client:

```bash
cargo run -- client \
  --connect 127.0.0.1:4433 \
  --bind 127.0.0.1:0
```

Expected flow:

1. the server starts a Quinn endpoint;
2. the client starts a Quinn endpoint and creates a logical QuicVid session;
3. the client establishes a QUIC connection;
4. the client sends `HELLO <session-id>` over a bidirectional QUIC stream;
5. the server validates the session and replies with `OK <session-id>`;
6. the client sends generated fake-video frames over QUIC DATAGRAMs;
7. the client sends `DONE <session-id> <frame-count>` after the media run;
8. the server drains remaining in-flight media and calculates delivery statistics;
9. the server replies with `DONE_OK <session-id>`;
10. the client closes the connection cleanly.

## Fake-video QUIC demo

QuicVid can generate a finite stream of synthetic video-like frames and send
them using QUIC application DATAGRAMs.

Each fake frame:

- has a monotonically increasing frame ID;
- uses the logical QuicVid session UUID;
- carries a sender timestamp;
- uses `chunk_index = 0` and `chunk_count = 1`;
- contains a deterministic generated payload.

The media framing already supports multiple chunks per logical frame so it can
later be reused for encoded visible video.

### Run the server

```bash
cargo run -- server \
  --listen 127.0.0.1:4433
```

### Run the client

```bash
cargo run -- client \
  --connect 127.0.0.1:4433 \
  --bind 127.0.0.1:0 \
  --fps 30 \
  --duration-seconds 10 \
  --payload-size 256
```

This configuration generates:

```text
30 FPS × 10 seconds = 300 frames
```

After all frames have been generated, the client sends:

```text
DONE <session-id> <frame-count>
```

The server briefly drains any remaining in-flight DATAGRAMs, calculates the
delivery statistics, and responds with:

```text
DONE_OK <session-id>
```

The client closes the connection only after receiving this acknowledgement.

### Receive summary

The server reports:

* `expected`: number of frames reported by the sender;
* `received`: total accepted fake-frame DATAGRAMs, including duplicates;
* `unique`: number of distinct frame IDs;
* `missing`: expected frames not received;
* `out_of_order`: previously unseen frame IDs arriving below the highest frame
  ID already observed;
* `duplicates`: repeated frame IDs;
* `largest_gap_ms`: largest server-side gap between consecutive accepted frame
  arrivals.

### Media datagram format

```text
Offset  Size  Field
0       1     media message type
1       16    session UUID
17      8     frame ID
25      8     sender timestamp (ms)
33      2     chunk index
35      2     chunk count
37      N     media payload
```

The fake-video workload currently uses one DATAGRAM per logical frame:

```text
chunk_index = 0
chunk_count = 1
```

Larger encoded video frames can later reuse the same format by splitting one
logical frame across several DATAGRAMs with the same frame ID.

### Verified local baseline

A localhost baseline run using:

```text
30 FPS
10 seconds
256-byte payload
300 expected frames
```

produced:

```text
expected=300
received=300
unique=300
missing=0
out_of_order=0
duplicates=0
largest_gap_ms=48
```

This result is only a local functional baseline, not a network-performance result. Later controlled Mininet experiments will introduce path changes and network impairment for evaluation.

### DATAGRAM behavior

QUIC DATAGRAMs are used for the media workload because delivery is unreliable
and unordered, which is closer to real-time media requirements than reliable
stream retransmission.

The sender checks Quinn's current maximum DATAGRAM size before streaming and
rejects single-chunk payloads that exceed the available media payload size.

## Mininet connection migration

Controlled QUIC connection migration is demonstrated using a dual-path
Mininet topology and the migration demo launcher.

From the repository root:

```bash
cargo build --release --manifest-path quic-vid/Cargo.toml
sudo mn -c
sudo python3 scripts/mininet/migration_demo.py --preset preview
