// quiche_server.c

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <fcntl.h>
#include <errno.h>
#include <time.h>
#include "quiche.h"

#define MAX_DATAGRAM_SIZE 1350
#define LOCAL_CONN_ID_LEN 16
#define MAX_CONNECTIONS 64

// Structure to hold a connection and its DCID for lookup
struct Connection {
    uint8_t dcid[QUICHE_MAX_CONN_ID_LEN];
    size_t dcid_len;
    quiche_conn *conn;
    struct sockaddr_storage last_peer;
    socklen_t last_peer_len;
};

// Global Connection Map
struct Connection conn_map[MAX_CONNECTIONS] = {0};

/* -------------------- Helper Functions for Connection Map -------------------- */

// Function to find an existing connection by its DCID
struct Connection* find_connection(const uint8_t *dcid, size_t dcid_len) {
    for (int i = 0; i < MAX_CONNECTIONS; i++) {
        if (conn_map[i].conn != NULL && 
            conn_map[i].dcid_len == dcid_len &&
            memcmp(conn_map[i].dcid, dcid, dcid_len) == 0) {
            return &conn_map[i];
        }
    }
    return NULL;
}

// Function to add a new connection to the map
struct Connection* add_connection(quiche_conn *conn, const uint8_t *dcid, size_t dcid_len) {
    for (int i = 0; i < MAX_CONNECTIONS; i++) {
        if (conn_map[i].conn == NULL) {
            conn_map[i].conn = conn;
            conn_map[i].dcid_len = dcid_len;
            memcpy(conn_map[i].dcid, dcid, dcid_len);
            // Initialize peer info
            memset(&conn_map[i].last_peer, 0, sizeof(conn_map[i].last_peer));
            conn_map[i].last_peer_len = 0;
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
        printf("Connection removed\n");
    }
}

// Function to format DCID into a printable string
void print_cid(const uint8_t *cid, size_t len) {
    for (size_t i = 0; i < len; i++) {
        printf("%02x", cid[i]);
    }
}

/* -------------------- Setup Functions -------------------- */

quiche_config* init_quiche_config(void) {
    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (!config) return NULL;

    if (quiche_config_load_cert_chain_from_pem_file(config, "cert.pem") != 0) return NULL;
    if (quiche_config_load_priv_key_from_pem_file(config, "key.pem") != 0) return NULL;

    quiche_config_set_application_protos(config, (uint8_t*)"\x08http/0.9", 9);
    quiche_config_set_max_idle_timeout(config, 30000);
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);
    quiche_config_set_initial_max_streams_uni(config, 100);
    quiche_config_set_disable_active_migration(config, false);

    return config;
}

int setup_socket(uint16_t port) {
    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) return -1;

    fcntl(sock, F_SETFL, O_NONBLOCK);

    struct sockaddr_in addr = {0};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = INADDR_ANY;

    if (bind(sock, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
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

    while ((sent = quiche_conn_send(conn, out, MAX_DATAGRAM_SIZE, &si)) > 0) {
        if (!logged_send) {
            printf("TX pending   | Conn ID: "); // Log only once
            print_cid(scid, scid_len);
            printf("\n");
            logged_send = true;
        }
        sendto(sock, out, sent, 0, (struct sockaddr*)&si.to, si.to_len);
    }
}

void handle_connection(struct Connection *c, uint8_t *buf, ssize_t len,
                       struct sockaddr_storage *peer, socklen_t peer_len,
                       struct sockaddr_in *local, int sock, uint8_t *out) {

    quiche_recv_info ri = {
        .from = (struct sockaddr*)peer,
        .from_len = peer_len,
        .to = (struct sockaddr*)local,
        .to_len = sizeof(*local)
    };

    ssize_t done = quiche_conn_recv(c->conn, buf, len, &ri);
    if (done >= 0) {
        // Save the latest peer address (for migration)
        memcpy(&c->last_peer, peer, peer_len);
        c->last_peer_len = peer_len;
    } else {
        fprintf(stderr, "quiche_conn_recv failed: %zd\n", done);
        return;
    }

    if (quiche_conn_is_established(c->conn)) {
        quiche_stream_iter *r = quiche_conn_readable(c->conn);
        uint64_t s;
        while (quiche_stream_iter_next(r, &s)) {
            uint8_t data[4096];
            bool fin;
            // Use c->conn
            ssize_t n = quiche_conn_stream_recv(c->conn, s, data, sizeof(data), &fin, NULL); 
            if (n > 0) {
                printf("   ---> RX Stream %lu: %.*s\n", (unsigned long)s, (int)n, data);

                char response[4096];
                int resp_len = snprintf(response, sizeof(response), "ECHO: %.*s", (int)n, data);
                // Use c->conn
                quiche_conn_stream_send(c->conn, s, (uint8_t*)response, resp_len, false, NULL); 
                printf("   <--- TX Stream %lu: Echo response sent\n", (unsigned long)s);
            }
        }
        quiche_stream_iter_free(r);
    }

    send_pending(c->conn, sock, out, &c->last_peer, c->last_peer_len,
                     c->dcid, c->dcid_len);

    if (quiche_conn_is_closed(c->conn)) {
        remove_connection(c);
    }
}

/* -------------------- Main Loop -------------------- */

int run_server(int port) {
    srand(time(NULL));

    quiche_config *config = init_quiche_config();
    if (!config) {
        fprintf(stderr, "Failed to initialize quiche config\n");
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
    getsockname(sock, (struct sockaddr*)&local, &local_len);
    
    // The main loop now iterates over all connections to handle timeouts/pending sends
    while (1) {
        struct sockaddr_storage peer;
        socklen_t peer_len = sizeof(peer);

        // Check if there are any packets to read
        ssize_t len = recvfrom(sock, buf, sizeof(buf), 0, (struct sockaddr*)&peer, &peer_len);

        if (len < 0) {
            if (errno != EAGAIN && errno != EWOULDBLOCK) {
                perror("recvfrom");
            } else {
                // If no packet, check all existing connections for pending data or timeouts
                for (int i = 0; i < MAX_CONNECTIONS; i++) {
                    if (conn_map[i].conn != NULL) {
                        // Check for timeout
                        uint64_t timeout = quiche_conn_timeout_as_nanos(conn_map[i].conn);
                        if (timeout == 0) {
                             printf("Connection timed out\n");
                             remove_connection(&conn_map[i]);
                             continue;
                        }

                        // Send pending
                        send_pending(conn_map[i].conn, sock, out, &conn_map[i].last_peer,
                            conn_map[i].last_peer_len, conn_map[i].dcid, conn_map[i].dcid_len);
                        
                        // Check if connection was closed after sending
                        if (quiche_conn_is_closed(conn_map[i].conn)) {
                             remove_connection(&conn_map[i]);
                        }
                    }
                }
            }
            // Sleep briefly to avoid busy-waiting
            usleep(1000); 
            continue;
        }

        uint8_t type, scid[QUICHE_MAX_CONN_ID_LEN], dcid[QUICHE_MAX_CONN_ID_LEN];
        uint8_t token[256];
        size_t scid_len = sizeof(scid), dcid_len = sizeof(dcid), token_len = sizeof(token);
        uint32_t version;

        // Parse the header to get the DCID (the connection ID the client thinks the server has)
        if (quiche_header_info(buf, len, LOCAL_CONN_ID_LEN, &version, &type,
                               scid, &scid_len, dcid, &dcid_len,
                               token, &token_len) < 0) {
            fprintf(stderr, "Failed to parse header\n");
            continue;
        }

        // --- Connection Lookup Logic (The Fix) ---
        struct Connection *c = find_connection(dcid, dcid_len);

        if (c != NULL) {
            // Case 1: Existing Connection found
            // This handles subsequent packets for an established connection
            printf("RX %zd bytes | Conn ID: ", len);
            print_cid(c->dcid, c->dcid_len);
            printf("\n");
            handle_connection(c, buf, len, &peer, peer_len, &local, sock, out);
        } else if (quiche_version_is_supported(version)) {
            // Case 2: No existing connection found, attempt to accept a new one
            // This handles the Initial packet for a new connection
            uint8_t new_scid[LOCAL_CONN_ID_LEN];
            for (int i = 0; i < LOCAL_CONN_ID_LEN; i++) new_scid[i] = rand() % 256;

            quiche_conn *new_conn = quiche_accept(new_scid, LOCAL_CONN_ID_LEN, dcid, dcid_len,
                                                 (struct sockaddr*)&local, sizeof(local),
                                                 (struct sockaddr*)&peer, peer_len, config);

            if (!new_conn) {
                fprintf(stderr, "Failed to create connection\n");
                continue;
            }
            printf("New Connection established | Conn ID: ");
            print_cid(new_scid, LOCAL_CONN_ID_LEN);
            printf("\n");
            
            // Add the new connection to the map
            c = add_connection(new_conn, new_scid, LOCAL_CONN_ID_LEN);
            
            if (c != NULL) {
                // Now handle the received packet for the new connection
                handle_connection(c, buf, len, &peer, peer_len, &local, sock, out);
            }
        } else {
            // Case 3: Version not supported or other unhandled packet
            fprintf(stderr, "Dropping packet: Unknown DCID or unsupported version\n");
        }
    }

    // Cleanup
    close(sock);
    quiche_config_free(config);
    return 0;
}

int main() {
    return run_server(4433);
}