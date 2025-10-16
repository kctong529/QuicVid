# Exploring Quiche: A QUIC Protocol Implementation Guide

## Introduction
Quiche is Cloudflare's implementation of the QUIC transport protocol and HTTP/3. This guide documents a hands-on exploration of the quiche tools provided as part of the quiche-apps crate, demonstrating how to set up and test both client and server applications.

## Building and Getting Started

### Prerequisites
- Rust toolchain installed
- Git installed
- Basic understanding of networking concepts

### Initial Setup
Following Cloudflare's official instructions, clone and build the quiche project:

```bash
git clone --recursive https://github.com/cloudflare/quiche
cd quiche
cargo build --examples
```

## Testing with Cloudflare's Public QUIC Server

### Running the Client
The simplest test is connecting to Cloudflare's public QUIC test server:

```bash
cargo run --bin quiche-client -- https://cloudflare-quic.com/
```

**Expected Output:** You'll receive a static HTML page with connection details.

### Understanding the Response
When connecting to cloudflare-quic.com, receiving static HTML is **expected behavior**. This indicates:
- The quiche-client successfully established a QUIC connection
- The client properly retrieved the test page
- Your client implementation is working correctly

The cloudflare-quic.com endpoint is Cloudflare's public QUIC test server specifically designed to return a basic HTML page demonstrating successful QUIC connectivity.

## Setting Up Your Own QUIC Server

### Basic Server Setup
Start a local quiche server using the provided self-signed certificate:

```bash
cargo run --bin quiche-server -- --cert apps/src/bin/cert.crt --key apps/src/bin/cert.key
```

**Note:** The certificate provided is self-signed and should not be used in production.

### Testing Client-Server Communication

#### 1. Start the Local Server
Specify a custom listening address:

```bash
cargo run --bin quiche-server -- --cert apps/src/bin/cert.crt --key apps/src/bin/cert.key --listen 127.0.0.1:4433
```

#### 2. Verify Server is Listening
Check that your server is properly bound to the port:

```bash
netstat -an | grep 4433
```

Expected output:
```
udp4       0      0  127.0.0.1.4433         *.*
```

#### 3. Connect Client to Local Server
Connect to your local server (note the `--no-verify` flag for self-signed certificates):

```bash
cargo run --bin quiche-client -- https://127.0.0.1:4433/ --no-verify
```

**Initial Result:** You'll likely see:
```
Not Found!
```

This is expected! The server is running but has no content to serve yet.

## Serving Content with Your QUIC Server

### Creating Web Content
The quiche-server needs a document root directory with files to serve. Let's create one:

```bash
# Create a web root directory
mkdir -p web-root

# Create an HTML index file
echo "<h1>QUIC Server Test</h1><p>It works!</p>" > web-root/index.html

# Create a test text file
echo "Test file content" > web-root/test.txt
```

### Running the Server with Content
Start the server with the document root specified:

```bash
cargo run --bin quiche-server -- --cert apps/src/bin/cert.crt --key apps/src/bin/cert.key --listen 127.0.0.1:4433 --root ./web-root
```

### Testing Content Retrieval

#### Fetch the Index Page
```bash
cargo run --bin quiche-client -- https://127.0.0.1:4433/ --no-verify
```

**Output:**
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.32s
Running `target/debug/quiche-client 'https://127.0.0.1:4433/' --no-verify`
<h1>QUIC Server Test</h1><p>It works!</p>
```

#### Fetch a Specific File
```bash
cargo run --bin quiche-client -- https://127.0.0.1:4433/test.txt --no-verify
```

**Output:**
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.12s
Running `target/debug/quiche-client 'https://127.0.0.1:4433/test.txt' --no-verify`
Test file content
```

## Next Steps

- Use the C bindings to establish QUIC connections programmatically

## Important Notes

- The provided certificate is self-signed and should **not** be used in production
- QUIC uses UDP instead of TCP (hence `udp4` in netstat output)
- The `--no-verify` flag bypasses certificate validation - only use in development
