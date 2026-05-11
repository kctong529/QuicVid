# Debugging QUIC Connection Migration Error -18

## The Problem

Error `-18` (Path validation failure) occurs immediately when calling `quiche_conn_probe_path()`. This happens before any packets are sent, meaning quiche is rejecting the probe attempt at the API level.

## Common Causes of Error -18

### 1. **Existing Path Validation In Progress**
Quiche only allows one path validation at a time. If there is already one ongoing, it will reject new probes with -18.

**Check:** Look for any previous migration that did not complete

### 2. **The "New" Path Is Actually The Same as Current Path**
When both sockets bind to `0.0.0.0:ephemeral_port`, quiche might see them as the same "path" even though they have different ports.

**Why this matters:** In QUIC, a "path" is defined by the 4-tuple:
- Local IP
- Local Port
- Remote IP
- Remote Port

If the connection is on loopback (127.0.0.1) and just changing the local port, some implementations might not see this as a sufficiently different path.

### 3. **No Available DCIDs**
The server needs to have sent NEW_CONNECTION_ID frames before migration can occur. If the DCID pool is empty, the probe will fail.

**Check:** `quiche_conn_available_dcids(conn)` should be greater than 0

### 4. **Connection Not in Right State**
Migration might be disabled, or the connection might be draining or closing.

## Debugging Steps

### Step 1: Run the Debug Version

The `client_debug.c` file adds extensive logging:

```bash
gcc -o client_debug client_debug.c -lquiche -lm
./client_debug
```

When Ctrl-C is pressed, it will print:
- Old local address (IP:port)
- New local address (IP:port)
- Peer address
- Available DCIDs
- Active SCIDs
- Connection state

### Step 2: Check the Output

**Look for these specific issues:**

```
DEBUG: Old local: 127.0.0.1:54321
DEBUG: New local: 127.0.0.1:54322
```
If both show 127.0.0.1, this might be the problem (loopback-only migration)

```
DEBUG: Available DCIDs: 0
```
If this is 0, migration cannot proceed yet

```
DEBUG: Active SCIDs: 1
```
If this is 1, the server has not issued additional SCIDs

### Step 3: Check Server Configuration

In the `server.c` file, verify:

```c
// This MUST be false (migration enabled)
quiche_config_set_disable_active_migration(config, false);

// This should be >= 2 (allow multiple CIDs)
quiche_config_set_active_connection_id_limit(config, 8);
```

Ensure the server is actually issuing NEW_CONNECTION_ID frames:
```c
if (quiche_conn_is_established(c->conn)) {
    size_t active_scids = quiche_conn_active_scids(c->conn);
    if (active_scids < ACTIVE_CONNECTION_ID_LIMIT / 2) {
        issue_new_scid(c);
    }
}
```

## Possible Solutions

### Solution 1: Wait for Server to Send More CIDs

Before triggering migration, check:
```c
while (quiche_conn_available_dcids(conn) == 0) {
    // Send ACK-eliciting packet to prompt server to send NEW_CONNECTION_ID
    quiche_conn_send_ack_eliciting(conn);
    send_pending_packets(conn, &peer, &peer_len, out);
    // Wait and receive packets...
}
```

### Solution 2: Test with Real Network Interfaces

Instead of loopback testing, try with actual network interfaces:

```bash
# Find local IP
ip addr show

# Run server bound to specific IP
./server

# Run client with real server IP (not 127.0.0.1)
./client 192.168.1.100
```

When Ctrl-C is pressed, the client will attempt to find a different local interface for migration.

### Solution 3: Use quiche_conn_migrate_source() Instead

If the goal is to change the local address without path validation:

```c
// This is simpler but less safe (no validation)
int ret = quiche_conn_migrate_source(conn, 
                                     (struct sockaddr *)&new_local_addr,
                                     sizeof(new_local_addr),
                                     &seq);
```

**Note:** This skips path validation, so should only be used on trusted networks.

### Solution 4: Check quiche Version

Error -18 behavior might differ between quiche versions. Check the version:

```bash
pkg-config --modversion quiche
```

Recent versions (0.18+) have better migration support.

## What The Debug Output Should Show

When working correctly, the output should show:

```
=== Simulating IP change via Ctrl-C ===
DEBUG: Old local: 127.0.0.1:54321
DEBUG: New local: 127.0.0.1:54322
DEBUG: Peer: 127.0.0.1:4433
DEBUG: Available DCIDs: 3
DEBUG: Active SCIDs: 4
DEBUG: Calling quiche_conn_probe_path()...
=== Migration Probe Started (seq=1) ===
Probing new path, waiting for validation...
Still using old socket for sending
```

If error -18 appears, examine the DEBUG output immediately above it to identify which condition failed.

## Testing Strategy

1. **First:** Ensure the server is issuing SCIDs
   - Add logging in server's `issue_new_scid()` function
   - Verify it is being called regularly

2. **Second:** Verify the client has DCIDs available
   - Should see "Available DCIDs: 3" or higher before migration

3. **Third:** Try with actual different interfaces
   - Use WiFi vs Ethernet
   - Use two different VPN tunnels
   - Avoid relying on loopback-only testing

4. **Fourth:** Check quiche source code
   - Look at conditions for returning -18 in `quiche/src/lib.rs`
   - May reveal additional constraints

## Alternative: Manual Testing

Path validation can be manually tested by:

1. Starting the client on one network interface
2. Physically changing the network (WiFi to Ethernet)
3. Forcing socket rebind
4. Attempting migration

This approach simulates a real-world scenario better than same-interface testing.
