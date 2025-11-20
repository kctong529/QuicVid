#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <fcntl.h>
#include <errno.h>
#include <time.h>
#include <sys/time.h>
#include "quiche.h"
#include <signal.h>
#include <stdatomic.h>
#include <sys/socket.h>

#define MAX_DATAGRAM_SIZE 1350

// Global state
atomic_bool ip_change_requested = false;
int client_sock = -1;

void sigint_handler(int sig) {
    ip_change_requested = true;
}

/* -------------------- Helpers -------------------- */

void print_local_addr(int sock) {
    // Only print IPv4 if that's what we are using
    struct sockaddr_in local;
    socklen_t len = sizeof(local);
    if (getsockname(sock, (struct sockaddr *)&local, &len) == 0) {
        char buf[INET_ADDRSTRLEN];
        inet_ntop(AF_INET, &local.sin_addr, buf, sizeof(buf));
        printf("Current local address: %s:%d\n", buf, ntohs(local.sin_port));
    }
}

int setup_udp_socket(struct sockaddr_in *local) {
    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) return -1;

    fcntl(sock, F_SETFL, O_NONBLOCK);

    local->sin_family = AF_INET;
    local->sin_addr.s_addr = INADDR_ANY;
    local->sin_port = 0; // ephemeral port

    if (bind(sock, (struct sockaddr *)local, sizeof(*local)) < 0) {
        close(sock);
        return -1;
    }

    return sock;
}

quiche_config* init_quiche_config(void) {
    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (!config) return NULL;

    quiche_config_set_application_protos(config, (uint8_t *)"\x08http/0.9", 9);
    quiche_config_verify_peer(config, false);
    quiche_config_set_max_idle_timeout(config, 30000);
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);

    return config;
}

// Sends pending packets and detects/updates the peer address if migration occurred
void send_pending_packets(quiche_conn *conn, struct sockaddr_storage *peer, socklen_t *peer_len, uint8_t *out) {
    quiche_send_info si;
    ssize_t sent;
    
    while ((sent = quiche_conn_send(conn, out, MAX_DATAGRAM_SIZE, &si)) > 0) {
        const struct sockaddr *new_addr_ptr = (const struct sockaddr *)&si.to; 
        
        if (memcmp((const void *)peer, (const void *)new_addr_ptr, si.to_len) != 0) {
            
            struct sockaddr_in *old_peer_in = (struct sockaddr_in *)peer;
            struct sockaddr_in *new_peer_in = (struct sockaddr_in *)new_addr_ptr;
            
            char old_addr[INET_ADDRSTRLEN], new_addr[INET_ADDRSTRLEN];
            
            inet_ntop(AF_INET, &old_peer_in->sin_addr, old_addr, sizeof(old_addr));
            inet_ntop(AF_INET, &new_peer_in->sin_addr, new_addr, sizeof(new_addr));
            
            printf("--- Peer Address Migration Detected ---\n");
            printf("Server migrated from %s:%d to %s:%d\n",
                   old_addr, ntohs(old_peer_in->sin_port),
                   new_addr, ntohs(new_peer_in->sin_port));
            
            memcpy((void *)peer, (const void *)new_addr_ptr, si.to_len);
            *peer_len = si.to_len;
        }

        sendto(client_sock, out, sent, 0, new_addr_ptr, si.to_len);
    }
}

// Simulates an IP change by creating a new socket and migrating the source address
void perform_ip_change_migration(quiche_conn* conn, struct sockaddr_storage *local_ss, struct sockaddr_storage *peer_ss) {
    // 1. Create a new local address/socket using an IPv4-specific struct
    struct sockaddr_in new_local = {0};
    int new_sock = setup_udp_socket(&new_local);
    if (new_sock < 0) {
        perror("socket");
        return;
    }

    socklen_t sl = sizeof(new_local);
    uint64_t seq;
    
    // 2. Migrate the source address
    if (quiche_conn_migrate_source(conn, (struct sockaddr*)&new_local, sl, &seq) != 0) {
        fprintf(stderr, "migrate_source failed\n");
        close(new_sock);
        return;
    }

    // 3. Update global state and the main local storage variable
    client_sock = new_sock;
    memcpy(local_ss, &new_local, sizeof(new_local));
    
    print_local_addr(client_sock);

    printf("Migration requested (seq=%llu). Sending on new socket.\n",
           (unsigned long long)seq);
}

/* -------------------- Main Client Loop -------------------- */
int run_client(int argc, char *argv[]) {
    const char *server_addr = "127.0.0.1";
    if (argc >= 2) {
        server_addr = argv[1];
    }

    srand(time(NULL));
    signal(SIGINT, sigint_handler);

    quiche_config *config = init_quiche_config();
    if (!config) {
        fprintf(stderr, "Failed to init quiche config\n");
        return 1;
    }

    // Use generic sockaddr_storage for main address variables
    struct sockaddr_storage local_storage = {0}, peer = {0};

    // Use an IPv4 pointer for setup_udp_socket convenience
    struct sockaddr_in *local_in = (struct sockaddr_in *)&local_storage; 
    struct sockaddr_in *peer_in = (struct sockaddr_in *)&peer;
    
    client_sock = setup_udp_socket(local_in);

    if (client_sock < 0) {
        perror("socket");
        return 1;
    }

    socklen_t local_len = sizeof(struct sockaddr_in); 
    getsockname(client_sock, (struct sockaddr *)&local_storage, &local_len);

    // Initialize peer address in the storage variable
    peer_in->sin_family = AF_INET;
    peer_in->sin_port = htons(4433);
    if (inet_pton(AF_INET, server_addr, &peer_in->sin_addr) != 1) {
        fprintf(stderr, "Invalid server address: %s\n", server_addr);
        return 1;
    }

    uint8_t scid[QUICHE_MAX_CONN_ID_LEN];
    for (int i = 0; i < sizeof(scid); i++) scid[i] = rand() % 256;

    socklen_t peer_len = sizeof(struct sockaddr_in);
    quiche_conn *conn = quiche_connect(server_addr, scid, sizeof(scid),
                                       (struct sockaddr *)&local_storage, local_len,
                                       (struct sockaddr *)&peer, peer_len,
                                       config);
    if (!conn) {
        fprintf(stderr, "Failed to create connection\n");
        return 1;
    }

    printf("Ping client started\n");
    print_local_addr(client_sock);

    uint8_t out[MAX_DATAGRAM_SIZE], buf[65535];
    quiche_send_info si;
    ssize_t sent;

    send_pending_packets(conn, &peer, &peer_len, out);

    struct timeval last_ping = {0}, ping_sent_time = {0};
    uint64_t stream_id = 0;
    int ping_seq = 0;
    bool waiting_for_pong = false;

    while (!quiche_conn_is_closed(conn)) {
        if (ip_change_requested) {
            ip_change_requested = false;
            printf("Simulating local IP change via Ctrl-C…\n");
            // Pass the address of the storage structs
            perform_ip_change_migration(conn, &local_storage, &peer);
        }

        ssize_t len = recvfrom(client_sock, buf, sizeof(buf), 0, NULL, NULL);
        if (len > 0) {
            quiche_recv_info ri = {
                .from = (struct sockaddr *)&peer,
                .from_len = peer_len,
                .to = (struct sockaddr *)&local_storage,
                .to_len = local_len
            };
            quiche_conn_recv(conn, buf, len, &ri);
            send_pending_packets(conn, &peer, &peer_len, out);
        } else if (len < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
            perror("recvfrom");
        }

        // Ping logic
        if (quiche_conn_is_established(conn)) {
            struct timeval now;
            gettimeofday(&now, NULL);

            if (!waiting_for_pong && (now.tv_sec - last_ping.tv_sec >= 1 || last_ping.tv_sec == 0)) {
                char ping_msg[64];
                snprintf(ping_msg, sizeof(ping_msg), "PING %d", ping_seq);
                stream_id = 0;
                quiche_conn_stream_send(conn, stream_id, (uint8_t *)ping_msg, strlen(ping_msg), false, NULL);

                gettimeofday(&ping_sent_time, NULL);
                printf("Sent: %s\n", ping_msg);
                last_ping = now;
                waiting_for_pong = true;
                ping_seq++;
            }

            if (waiting_for_pong) {
                struct timeval now;
                gettimeofday(&now, NULL);

                long rtt_us = (now.tv_sec - ping_sent_time.tv_sec) * 1000000 +
                              (now.tv_usec - ping_sent_time.tv_usec);

                if (rtt_us > 2000000) {
                    printf("Pong timeout, try to detect migration\n");
                    waiting_for_pong = false;
                    perform_ip_change_migration(conn, &local_storage, &peer);
                } else {
                    uint8_t data[4096];
                    bool fin;
                    ssize_t n = quiche_conn_stream_recv(conn, stream_id, data, sizeof(data), &fin, NULL);
                    if (n > 0) {
                        printf("Received: %.*s (RTT: %.2f ms)\n", (int)n, data, rtt_us / 1000.0);
                        waiting_for_pong = false;
                    }
                }
            }
        }

        send_pending_packets(conn, &peer, &peer_len, out);
        usleep(1000);
    }

    printf("Connection closed\n");
    quiche_conn_free(conn);
    quiche_config_free(config);
    close(client_sock);
    return 0;
}

int main(int argc, char *argv[]) {
    return run_client(argc, argv);
}