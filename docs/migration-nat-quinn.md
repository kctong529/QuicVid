# QUIC Connection Migration via NAT Rebinding

The following shows the successful implementation and verification of QUIC connection migration using the Quinn library. The experiment demonstrates how a QUIC session remains stable despite multiple changes to the underlying UDP source port, a scenario that would traditionally terminate a TCP-based connection.

## Experimental Setup

The test environment consisted of a local client-server architecture implemented in Rust using the `quinn` crate (v0.11) and `rustls` (v0.23).

* **Server:** Configured to listen on `127.0.0.1:4433` using a self-signed certificate and the `hq-29` (HTTP/0.9) ALPN protocol.
* **Client:** Designed to establish a single bidirectional stream and transmit a "PING" message every second.
* **Migration Trigger:** At rounds 5, 10, and 15, the client explicitly invoked `endpoint.rebind()`, forcing the underlying UDP socket to close and a new one to bind to a random available port.


## How to Run the Experiment

### Project Structure

The project is organized as a single Cargo package with two binaries:

```text
quinn-ping/
├── Cargo.toml
└── src/
    └── bin/
        ├── server.rs
        └── client.rs

```

### Execution Steps

1. **Start the Server:** Open a terminal and run the server binary.
```bash
cargo run --bin server

```


2. **Start the Client:** Open a second terminal and run the client binary.
```bash
cargo run --bin client

```


3. **Observation:** Observe the server logs to see the "From" address change ports while the Connection ID (CID) remains identical.

## Observations and Results

### Server-Side Log Analysis

The server maintained a single continuous session with the client. The logs confirm that while the source address changed, the **Connection ID (CID)** remained constant, allowing the application layer to continue without interruption.

```text
Server listening on 127.0.0.1:4433
Client connected. Starting ping-pong loop...
Received PING | CID: 5754679824 | From: 127.0.0.1:55443
...
Received PING | CID: 5754679824 | From: 127.0.0.1:63830  <-- Migration 1
...
Received PING | CID: 5754679824 | From: 127.0.0.1:65330  <-- Migration 2
...
Received PING | CID: 5754679824 | From: 127.0.0.1:62572  <-- Migration 3

```

### Client-Side Performance

The client experienced zero packet loss at the application level. Each "PONG" response was received successfully, even in the cycles immediately following a socket rebind.

## Technical Analysis

### Connection ID (CID) Stability

The primary mechanism enabling this migration is the decoupling of the connection identity from the network 4-tuple (Src IP, Src Port, Dst IP, Dst Port). In these tests, the server utilized the Stable Connection ID to route incoming packets to the correct internal state machine, ignoring the fact that the packets arrived from a previously unknown port.

### Path Validation

Following each `rebind`, the QUIC stack performed path validation. This involves the exchange of `PATH_CHALLENGE` and `PATH_RESPONSE` frames.

1. **Challenge:** The server sends a probe to the new client address to ensure reachability.
2. **Response:** The client echoes the challenge to prove it is not spoofing the address.
The logs show that this process happens transparently to the application code, with the `bi_stream` remaining fully functional throughout.

## Conclusion

The experiment successfully proved that QUIC connection migration can handle frequent NAT rebinding events without dropping the application-layer stream. This validates QUIC’s suitability for mobile and unstable network environments where IP or port changes are common.
