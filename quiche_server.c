#include "quiche.h"
#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/select.h>
#include <time.h>
#include <unistd.h>
#include <stdbool.h>

#define MAX_DATAGRAM_SIZE 1350
#define LOCAL_CONN_ID_LEN 16
#define MAX_CONNECTIONS 64
#define ACTIVE_CONNECTION_ID_LIMIT 8 // Matches the value set in init_quiche_config

// Structure to hold a connection and its DCID for lookup
struct Connection {
    uint8_t dcid[QUICHE_MAX_CONN_ID_LEN];
    size_t dcid_len;
    quiche_conn *conn;
    struct sockaddr_storage last_peer;
    socklen_t last_peer_len;
    uint8_t current_scid[LOCAL_CONN_ID_LEN];
    size_t current_scid_len;
    size_t scids_issued; // Track how many SCIDs we've issued
};

// Global Connection Map
struct Connection conn_map[MAX_CONNECTIONS] = {0};

/* -------------------- Helper Functions for Connection Map --------------------
 */

// Function to find an existing connection by its DCID
struct Connection *find_connection(const uint8_t *dcid, size_t dcid_len) {
    for (int i = 0; i < MAX_CONNECTIONS; i++) {
        if (conn_map[i].conn != NULL && conn_map[i].dcid_len == dcid_len &&
            memcmp(conn_map[i].dcid, dcid, dcid_len) == 0) {
            return &conn_map[i];
        }
    }
    return NULL;
}

// Function to add a new connection to the map
struct Connection *add_connection(quiche_conn *conn, const uint8_t *dcid,
                                  size_t dcid_len,
                                  struct sockaddr_storage *initial_peer,
                                  socklen_t initial_peer_len) {
    for (int i = 0; i < MAX_CONNECTIONS; i++) {
        if (conn_map[i].conn == NULL) {
            conn_map[i].conn = conn;
            conn_map[i].dcid_len = dcid_len;
            memcpy(conn_map[i].dcid, dcid, dcid_len);
            conn_map[i].scids_issued = 0; // Initialize counter

            // Initialize peer info with the address that sent the packet
            memcpy(&conn_map[i].last_peer, initial_peer, initial_peer_len);
            conn_map[i].last_peer_len = initial_peer_len;

            return &conn_map[i];
        }
    }
    fprintf(stderr, "Error: Connection map full\n");
    quiche_conn_free(conn);
    return NULL;
}

// Function to remove a closed connection
void remove_connection(struct Connection *c) {
    if (c->conn) {
        quiche_conn_free(c->conn);
        c->conn = NULL;
        c->dcid_len = 0;
        c->scids_issued = 0;
        printf("Connection removed\n");
    }
}

// Function to format DCID into a printable string
void print_cid(const uint8_t *cid, size_t len) {
    for (size_t i = 0; i < len; i++) {
        printf("%02x", cid[i]);
    }
}

void issue_new_scid(struct Connection *c) {
    uint8_t new_scid[LOCAL_CONN_ID_LEN];
    uint8_t reset_token[16];
    uint64_t scid_seq;

    // Generate a new SCID and a random Stateless Reset Token
    for (int i = 0; i < LOCAL_CONN_ID_LEN; i++)
        new_scid[i] = rand() % 256;
    for (int i = 0; i < 16; i++)
        reset_token[i] = rand() % 256;

    // Call quiche API to register the new SCID and queue the NEW_CONNECTION_ID
    // frame
    int ret = quiche_conn_new_scid(c->conn, new_scid, LOCAL_CONN_ID_LEN,
                                   reset_token, false, &scid_seq);

    if (ret == 0) {
        // Update connection state tracking
        memcpy(c->current_scid, new_scid, LOCAL_CONN_ID_LEN);
        c->current_scid_len = LOCAL_CONN_ID_LEN;
        c->scids_issued++;
        printf("SERVER: Issued new SCID (Seq %llu, Total: %zu)\n",
               (unsigned long long)scid_seq, c->scids_issued);
    } else {
        fprintf(stderr, "SERVER: Failed to issue new SCID: %d\n", ret);
    }
}

/* -------------------- Setup Functions -------------------- */

quiche_config *init_quiche_config(void) {
    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (!config)
        return NULL;

    if (quiche_config_load_cert_chain_from_pem_file(config, "cert.pem") != 0)
        return NULL;
    if (quiche_config_load_priv_key_from_pem_file(config, "key.pem") != 0)
        return NULL;

    quiche_config_set_application_protos(config, (uint8_t *)"\x08http/0.9", 9);
    quiche_config_set_max_idle_timeout(config, 30000);
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);
    quiche_config_set_initial_max_streams_uni(config, 100);

    quiche_config_set_active_connection_id_limit(config, ACTIVE_CONNECTION_ID_LIMIT);
    // *** CRITICAL FIX: ENABLE CONNECTION MIGRATION ***
    quiche_config_set_disable_active_migration(config, false); 

    return config;
}

int setup_socket(uint16_t port) {
    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0)
        return -1;

    fcntl(sock, F_SETFL, O_NONBLOCK);

    struct sockaddr_in addr = {0};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = INADDR_ANY;

    if (bind(sock, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        close(sock);
        return -1;
    }

    return sock;
}

/* -------------------- Packet Processing -------------------- */

void send_pending(quiche_conn *conn, int sock, uint8_t *out,
                  struct sockaddr_storage *peer, socklen_t peer_len,
                  const uint8_t *scid, size_t scid_len) {

    quiche_send_info si;
    ssize_t sent;
    bool logged_send = false;

    // Note: si.to and si.to_len will automatically be the address determined by
    // the QUIC state machine. If the client migrated, this address will be the
    // new one.
    while ((sent = quiche_conn_send(conn, out, MAX_DATAGRAM_SIZE, &si)) > 0) {
        if (!logged_send) {
            printf("TX pending    | Conn ID: "); // Log only once
            print_cid(scid, scid_len);
            printf("\n");
            logged_send = true;
        }
        sendto(sock, out, sent, 0, (struct sockaddr *)&si.to, si.to_len);
    }
}

void handle_connection(struct Connection *c, uint8_t *buf, ssize_t len,
                        struct sockaddr_storage *peer, socklen_t peer_len,
                        struct sockaddr_in *local, int sock, uint8_t *out) {

    quiche_recv_info recv_info = {.from = (struct sockaddr *)peer,
                                 .from_len = peer_len,
                                 .to = (struct sockaddr *)local,
                                 .to_len = sizeof(*local)};

    // Call quiche_conn_recv first. If the packet contains a PATH_CHALLENGE from
    // a new address, quiche will process it and queue a PATH_RESPONSE.
    ssize_t read = quiche_conn_recv(c->conn, buf, len, &recv_info);
    if (read < 0 && read != QUICHE_ERR_DONE) {
        fprintf(stderr, "Connection %p recv failed: %zd\n", (void *)c->conn, read);
        return;
    }

    // Address Tracking: Update if peer address changed
    if (read > 0 || read == QUICHE_ERR_DONE) {
        if (memcmp(&c->last_peer, peer, peer_len) != 0) {
            
            struct sockaddr_in *new_peer_in = (struct sockaddr_in *)peer;
            char new_addr[INET_ADDRSTRLEN];
            inet_ntop(AF_INET, &new_peer_in->sin_addr, new_addr, sizeof(new_addr));

            printf("SERVER: Peer Address Changed! New Address: %s:%d\n", new_addr,
                   ntohs(new_peer_in->sin_port));

            memcpy(&c->last_peer, peer, peer_len);
            c->last_peer_len = peer_len;
        }
    }

    // 1. Check for connection establishment and manage SCID pool
    if (quiche_conn_is_established(c->conn)) {
        // Only issue new SCIDs when we're running low
        // Keep the pool above half capacity for migration flexibility
        size_t active_scids = quiche_conn_active_scids(c->conn);
        size_t threshold = ACTIVE_CONNECTION_ID_LIMIT / 2;
        
        // Issue SCIDs in small batches to avoid spam
        if (active_scids < threshold && c->scids_issued < ACTIVE_CONNECTION_ID_LIMIT) {
            issue_new_scid(c);
        }
        
        // 2. Handle readable streams
        quiche_stream_iter *r = quiche_conn_readable(c->conn);
        uint64_t stream_id = 0;

        while (quiche_stream_iter_next(r, &stream_id)) {
            uint8_t data[4096];
            bool fin;
            ssize_t n = quiche_conn_stream_recv(c->conn, stream_id, data,
                                                 sizeof(data), &fin, NULL);
            if (n > 0) {
                printf("    ---> RX Stream %lu: %.*s\n", (unsigned long)stream_id,
                       (int)n, data);

                char response[4096];
                int resp_len =
                    snprintf(response, sizeof(response), "PONG: %.*s", (int)n, data);
                quiche_conn_stream_send(c->conn, stream_id, (uint8_t *)response,
                                        resp_len, false, NULL);
                printf("    <--- TX Stream 0: PONG response queued\n");
            }
        }
        quiche_stream_iter_free(r);
    }

    // 3. Send pending packets (including PATH_RESPONSE and NEW_CONNECTION_ID)
    send_pending(c->conn, sock, out, &c->last_peer, c->last_peer_len, c->dcid,
                 c->dcid_len);

    // 4. Handle connection closure
    if (quiche_conn_is_closed(c->conn)) {
        printf("Connection %p closed.\n", (void *)c->conn);
    }
}

/* -------------------- Main Loop -------------------- */

int run_server(int port) {
    srand(time(NULL));

    quiche_config *config = init_quiche_config();
    if (!config) {
        fprintf(stderr, "Failed to initialize quiche config (Did you run 'openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 365' in this directory?)\n");
        return 1;
    }

    int sock = setup_socket(port);
    if (sock < 0) {
        perror("socket");
        quiche_config_free(config);
        return 1;
    }

    printf("Server listening on port %d\n", port);

    uint8_t buf[65535], out[MAX_DATAGRAM_SIZE];
    struct sockaddr_in local;
    socklen_t local_len = sizeof(local);
    getsockname(sock, (struct sockaddr *)&local, &local_len);

    while (1) {
        // --- Step 1: Calculate Minimum Timeout for select() ---
        uint64_t min_timeout_ns = 0;
        
        for (int i = 0; i < MAX_CONNECTIONS; i++) {
            if (conn_map[i].conn != NULL) {
                uint64_t timeout_ns = quiche_conn_timeout_as_nanos(conn_map[i].conn);
                if (timeout_ns == 0) {
                    min_timeout_ns = 0; // Immediate action needed
                    break;
                }
                
                if (min_timeout_ns == 0 || timeout_ns < min_timeout_ns) {
                    min_timeout_ns = timeout_ns;
                }
            }
        }

        // Convert the minimum timeout to a struct timeval for select.
        struct timeval timeout_tv;
        struct timeval *timeout_ptr = NULL;
        
        if (min_timeout_ns > 0) {
            uint64_t timeout_us = min_timeout_ns / 1000;
            if (timeout_us == 0) timeout_us = 1; // Minimum 1 microsecond wait
            
            timeout_tv.tv_sec = timeout_us / 1000000;
            timeout_tv.tv_usec = timeout_us % 1000000;
            timeout_ptr = &timeout_tv;
        }

        // --- Step 2: Use select() for Polling ---
        fd_set readfds;
        FD_ZERO(&readfds);
        FD_SET(sock, &readfds);

        int ret = select(sock + 1, &readfds, NULL, NULL, timeout_ptr);
        
        if (ret < 0) {
            if (errno != EINTR) {
                perror("select");
                break;
            }
        }
        
        // --- Step 3: Handle QUIC Timer Expiry or Packet Reception ---
        
        // Handle timer expiry for all connections
        for (int i = 0; i < MAX_CONNECTIONS; i++) {
            if (conn_map[i].conn != NULL) {
                if (quiche_conn_timeout_as_nanos(conn_map[i].conn) == 0) {
                    quiche_conn_on_timeout(conn_map[i].conn);
                    send_pending(conn_map[i].conn, sock, out, &conn_map[i].last_peer,
                                 conn_map[i].last_peer_len, conn_map[i].dcid,
                                 conn_map[i].dcid_len);
                }
            }
        }

        // Handle incoming packet
        if (ret > 0 && FD_ISSET(sock, &readfds)) {
            struct sockaddr_storage peer;
            socklen_t peer_len = sizeof(peer);
            ssize_t len = recvfrom(sock, buf, sizeof(buf), 0, (struct sockaddr *)&peer, &peer_len);

            if (len < 0) {
                if (errno != EAGAIN && errno != EWOULDBLOCK) {
                    perror("recvfrom");
                }
                continue;
            }

            uint8_t type, scid[QUICHE_MAX_CONN_ID_LEN], dcid[QUICHE_MAX_CONN_ID_LEN];
            uint8_t token[256];
            size_t scid_len = sizeof(scid), dcid_len = sizeof(dcid),
                   token_len = sizeof(token);
            uint32_t version;

            if (quiche_header_info(buf, len, LOCAL_CONN_ID_LEN, &version, &type, scid,
                                   &scid_len, dcid, &dcid_len, token, &token_len) < 0) {
                fprintf(stderr, "Failed to parse header\n");
                continue;
            }

            // --- Connection Lookup Logic ---
            struct Connection *c = find_connection(dcid, dcid_len);

            if (c != NULL) {
                // Existing Connection found
                printf("RX %zd bytes | Conn ID: ", len);
                print_cid(c->dcid, c->dcid_len);
                printf("\n");
                handle_connection(c, buf, len, &peer, peer_len, &local, sock, out);
            } else if (quiche_version_is_supported(version)) {
                // No existing connection found, accept new connection
                uint8_t new_scid[LOCAL_CONN_ID_LEN];
                for (int i = 0; i < LOCAL_CONN_ID_LEN; i++)
                    new_scid[i] = rand() % 256;

                quiche_conn *new_conn =
                    quiche_accept(new_scid, LOCAL_CONN_ID_LEN, dcid, dcid_len,
                                  (struct sockaddr *)&local, sizeof(local),
                                  (struct sockaddr *)&peer, peer_len, config);

                if (!new_conn) {
                    fprintf(stderr, "Failed to create connection\n");
                    continue;
                }
                printf("New Connection established | Conn ID: ");
                print_cid(new_scid, LOCAL_CONN_ID_LEN);
                printf("\n");

                c = add_connection(new_conn, new_scid, LOCAL_CONN_ID_LEN, &peer, peer_len);

                if (c != NULL) {
                    handle_connection(c, buf, len, &peer, peer_len, &local, sock, out);
                }
            } else {
                fprintf(stderr, "Dropping packet: Unknown DCID or unsupported version\n");
            }
        }
        
        // --- Step 4: Cleanup Closed Connections ---
        for (int i = 0; i < MAX_CONNECTIONS; i++) {
            if (conn_map[i].conn != NULL && quiche_conn_is_closed(conn_map[i].conn)) {
                remove_connection(&conn_map[i]);
            }
        }
    }

    // Cleanup
    close(sock);
    quiche_config_free(config);
    return 0;
}

int main() { return run_server(4433); }