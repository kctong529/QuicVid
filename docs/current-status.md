# Current Project Status

## Active direction

QuicVid is now a Quinn-based video-call prototype. New product work should use Quinn. Older quiche work is kept as legacy/background material.

## Existing prototypes

### quinn-ping

Purpose:
- Basic Quinn client/server prototype.
- Useful reference for Epic 1.2, especially Quinn endpoint setup, stream use, and connection/rebinding behavior.

Cargo package:
- `quinn-test`

Binaries:
- `client`
- `server`

Build status:
- Verified on: 2026-05-11
- Commands:

```bash
cd quinn-ping
cargo metadata --no-deps --format-version 1
cargo check
cargo build
```

Result:

* `cargo check` succeeds.
* `cargo build` succeeds.
* Warning: `src/bin/server.rs` has an unused import, `net::SocketAddr`.

Runtime status:
- Verified in Mininet on 2026-05-11.

Topology:
- `h1`: `10.0.0.1/24`
- `h2`: `10.0.0.2/24`
- server runs on `h1`
- client runs on `h2`

Commands:

```bash
sudo python3 scripts/mininet-two-hosts.py
```

Inside Mininet:

```bash
h1 bash -lc 'cd /home/ubuntu/QuicVid/quinn-ping && ./target/debug/server > /tmp/quinn-ping-server.log 2>&1 &'
h2 bash -lc 'cd /home/ubuntu/QuicVid/quinn-ping && ./target/debug/client > /tmp/quinn-ping-client.log 2>&1'
h1 cat /tmp/quinn-ping-server.log
h2 cat /tmp/quinn-ping-client.log
```

Result:

* server starts on `0.0.0.0:4433`
* client connects to `10.0.0.1:4433`
* client sends repeated `PING` messages
* server responds with `PONG`
* client triggers endpoint rebinding every 5 rounds
* server observes the peer source port change during the run
* the same stable QUIC session ID continues after rebinding

Observed example:

* server saw peer change from `10.0.0.2:35506` to `10.0.0.2:37098`
* stable session ID remained `275342434541600`

Conclusion:

* `quinn-ping` is a working Mininet runtime reference for Quinn client/server setup and endpoint rebinding.

### mouse-coordinates

Purpose:
- Quinn datagram prototype.
- Useful reference for Epic 1.3, especially fake-video datagram streaming.

Cargo package:
- `mouse-coordinates`

Binaries:
- `client`
- `server`

Build status:
- Verified on: 2026-05-11
- Commands:

```bash
cd mouse-coordinates
cargo metadata --no-deps --format-version 1
cargo check
cargo build
````

Result:

* `cargo check` succeeds.
* `cargo build` succeeds.

Runtime status:
- Verified in Mininet on 2026-05-11.

Topology:
- `h1`: `10.0.0.1/24`
- `h2`: `10.0.0.2/24`
- server runs on `h1`
- client runs on `h2`

Commands:

```bash
sudo python3 scripts/mininet-two-hosts.py
```

Inside Mininet:

```bash
h1 bash -lc 'cd /home/ubuntu/QuicVid/mouse-coordinates && ./target/debug/server'
h2 bash -lc 'cd /home/ubuntu/QuicVid/mouse-coordinates && ./target/debug/client'
```

Result:

* server starts on `0.0.0.0:4433`
* client connects to `10.0.0.1:4433`
* client sends coordinate datagrams
* server receives QUIC datagrams with `connection.read_datagram()`
* server displays the received coordinate position in the terminal
* client prints packet number, position, and RTT

Observed example:

* server displayed peer address `10.0.0.2:54545`
* client sent coordinate packets such as `Pkt 347`, `Pkt 348`, and later packets
* server displayed current position updates such as `Pos: 141,-160`

Caveat:

* the client reads real mouse input from `/dev/input/mice`
* this is Linux-specific and may require appropriate environment/permissions
* for the new `quic-vid` app, fake video frames should use generated frame data instead of OS mouse input

Conclusion:

* `mouse-coordinates` is a working Mininet runtime reference for Quinn datagram send/receive behavior.

## Legacy/background work

### quiche experiments

The C/quiche experiments and quiche notes are legacy/background material. They helped explore QUIC migration, path validation, and connection IDs, but new QuicVid product work should use Quinn.

### baseline-study

The baseline-study notes are useful project motivation, but the revised evaluation plan should use a smaller controlled baseline comparison.

## Local setup notes

Useful prototype checks:

```bash
cd quinn-ping
cargo check
cargo build

cd ../mouse-coordinates
cargo check
cargo build
```

## Next implementation step

Epic 1.2: create the main `quic-vid` app skeleton with server/client modes, Quinn setup, session IDs, an initial control-stream hello, and structured JSONL logs.
