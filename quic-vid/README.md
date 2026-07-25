# QuicVid

QuicVid is a Quinn-based prototype for experimenting with QUIC media transport
and connection migration.

The current application provides:

- separate client and server modes;
- configurable server and client addresses;
- configurable client local bind address;
- a logical QuicVid session UUID;
- a small control-stream handshake over QUIC;
- basic connection and address logging.

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
2. the client starts a Quinn endpoint;
3. the client establishes a QUIC connection;
4. the client generates a QuicVid session UUID;
5. the client sends `HELLO <session-id>` over a bidirectional QUIC stream;
6. the server validates and logs the same session ID;
7. the server replies with `OK <session-id>`;
8. the client validates the acknowledgement and closes cleanly.

Example client events:

```text
event=client_endpoint_created ...
event=session_created session=...
event=connecting ...
event=connected ...
event=hello_sent ...
event=hello_acknowledged ...
event=client_stopped ...
```

Example server events:

```text
event=server_started ...
event=client_connected ...
event=client_hello ...
event=hello_acknowledged ...
event=client_disconnected ...
```

## Development security

The current client accepts the self-signed development certificate without
normal certificate verification.

This is only intended for controlled local and Mininet experiments.

## Next step

Epic 1.3 adds numbered fake-video frames over QUIC datagrams while reusing the
same Quinn connection and QuicVid session.
