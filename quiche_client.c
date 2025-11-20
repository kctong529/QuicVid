#include "quiche.h"
#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/select.h>
#include <time.h>
#include <unistd.h>
#include <stdbool.h>

// Function definition must precede use if not declared in a header
const char *quiche_error_msg(int err);

#define MAX_DATAGRAM_SIZE 1350

// Global state
atomic_bool ip_change_requested = false;
int client_sock = -1;

// Migration tracking
bool migration_in_progress = false;
int old_migration_sock = -1;
struct sockaddr_in new_local_addr = {0};
struct timeval migration_start_time = {0};
uint64_t probe_seq = 0;
#define MIGRATION_TIMEOUT_US 5000000  // 5 seconds

void sigint_handler(int sig) { ip_change_requested = true; }

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
    if (sock < 0)
        return -1;

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

quiche_config *init_quiche_config(void) {
    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (!config)
        return NULL;

    quiche_config_set_application_protos(config, (uint8_t *)"\x08http/0.9", 9);
    quiche_config_verify_peer(config, false);
    quiche_config_set_max_idle_timeout(config, 10000);
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);
    quiche_config_set_initial_max_stream_data_uni(config, 1000000);
    
    quiche_config_set_active_connection_id_limit(config, 10); 

    return config;
}

// Sends pending packets and detects/updates the peer address if migration occurred
void send_pending_packets(quiche_conn *conn, struct sockaddr_storage *peer,
                          socklen_t *peer_len, uint8_t *out) {
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
            printf("Server migrated from %s:%d to %s:%d\n", old_addr,
                   ntohs(old_peer_in->sin_port), new_addr,
                   ntohs(new_peer_in->sin_port));

            memcpy((void *)peer, (const void *)new_addr_ptr, si.to_len);
            *peer_len = si.to_len;
        }

        sendto(client_sock, out, sent, 0, new_addr_ptr, si.to_len);
    }
}

// Start path probing for migration (step 1: probe the new path)
void start_migration_probe(quiche_conn *conn,
                           struct sockaddr_storage *local_ss,
                           socklen_t *local_len,
                           struct sockaddr_storage *peer_ss, uint8_t *out,
                           socklen_t *peer_len) {

    if (quiche_conn_is_closed(conn)) {
        fprintf(stderr, "Migration aborted: Connection is already closed.\n");
        return;
    }
    
    if (migration_in_progress) {
        fprintf(stderr, "Migration already in progress, please wait...\n");
        return;
    }
    
    // Create a new local address/socket
    new_local_addr.sin_family = AF_INET;
    new_local_addr.sin_addr.s_addr = INADDR_ANY;
    new_local_addr.sin_port = 0;
    
    int new_sock = setup_udp_socket(&new_local_addr);
    if (new_sock < 0) {
        perror("setup_udp_socket");
        return;
    }
    
    // Get the actual bound address
    socklen_t sl = sizeof(new_local_addr);
    getsockname(new_sock, (struct sockaddr *)&new_local_addr, &sl);
    
    // This starts path validation without immediately switching
    int ret = quiche_conn_probe_path(conn, (struct sockaddr *)&new_local_addr, 
                                     sizeof(new_local_addr),
                                     (struct sockaddr *)peer_ss, *peer_len, &probe_seq);
    
    if (ret != 0) {
        fprintf(stderr, "probe_path failed (QUICHE error code: %d, %s)\n", 
                ret, quiche_error_msg(ret));
        close(new_sock);
        return;
    }

    // Save the old socket
    old_migration_sock = client_sock;
    
    // Switch to new socket
    client_sock = new_sock;
    memcpy(local_ss, &new_local_addr, sizeof(new_local_addr));
    *local_len = sizeof(struct sockaddr_in);
    
    migration_in_progress = true;
    gettimeofday(&migration_start_time, NULL);
    
    printf("=== Migration Probe Started (seq=%llu) ===\n", (unsigned long long)probe_seq);
    printf("Probing new path, waiting for validation...\n");
    print_local_addr(client_sock);

    // Send PATH_CHALLENGE on the new socket
    send_pending_packets(conn, peer_ss, peer_len, out);
}

// Complete migration after path validation succeeds
void complete_migration(quiche_conn *conn, struct sockaddr_storage *local_ss,
                       socklen_t local_len, struct sockaddr_storage *peer_ss,
                       socklen_t peer_len) {
    if (!migration_in_progress) {
        return;
    }
    
    printf("=== Path Validation Successful! ===\n");
    
    // Now actually migrate to the validated path
    uint64_t migrate_seq;
    int ret = quiche_conn_migrate(conn, 
                                  (struct sockaddr *)&new_local_addr,
                                  sizeof(new_local_addr),
                                  (struct sockaddr *)peer_ss,
                                  peer_len,
                                  &migrate_seq);
    
    if (ret == 0) {
        printf("Migration complete (seq=%llu), closing old socket\n", 
               (unsigned long long)migrate_seq);
        
        if (old_migration_sock >= 0) {
            close(old_migration_sock);
            old_migration_sock = -1;
        }
        
        migration_in_progress = false;
    } else {
        fprintf(stderr, "Migration failed after validation: %d (%s)\n", 
                ret, quiche_error_msg(ret));
        
        // On failure, revert to old socket
        if (old_migration_sock >= 0) {
            close(client_sock);
            client_sock = old_migration_sock;
            old_migration_sock = -1;
        }
        migration_in_progress = false;
    }
}

// Check if migration has completed or timed out
void check_migration_status(quiche_conn *conn, struct sockaddr_storage *local_ss,
                           socklen_t local_len, struct sockaddr_storage *peer_ss,
                           socklen_t peer_len, uint8_t *out) {
    if (!migration_in_progress) {
        return;
    }
    
    struct timeval now;
    gettimeofday(&now, NULL);
    long elapsed_us = (now.tv_sec - migration_start_time.tv_sec) * 1000000 +
                      (now.tv_usec - migration_start_time.tv_usec);
    
    // Check if path is validated
    // Note: We'll detect this by successfully receiving packets on the new socket
    // or by checking if quiche_conn_migrate() succeeds
    
    // Check for timeout
    if (elapsed_us > MIGRATION_TIMEOUT_US) {
        fprintf(stderr, "Migration timeout after %.2f seconds - path validation failed\n",
                elapsed_us / 1000000.0);
        
        // Revert to old socket
        if (old_migration_sock >= 0) {
            close(client_sock);
            client_sock = old_migration_sock;
            old_migration_sock = -1;
            printf("Reverted to old socket\n");
            print_local_addr(client_sock);
        }
        migration_in_progress = false;
    }
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
        quiche_config_free(config);
        return 1;
    }

    socklen_t local_len = sizeof(struct sockaddr_in);
    getsockname(client_sock, (struct sockaddr *)&local_storage, &local_len);

    // Initialize peer address in the storage variable
    peer_in->sin_family = AF_INET;
    peer_in->sin_port = htons(4433);
    if (inet_pton(AF_INET, server_addr, &peer_in->sin_addr) != 1) {
        fprintf(stderr, "Invalid server address: %s\n", server_addr);
        quiche_config_free(config);
        close(client_sock);
        return 1;
    }

    uint8_t scid[QUICHE_MAX_CONN_ID_LEN];
    for (int i = 0; i < sizeof(scid); i++)
        scid[i] = rand() % 256;

    socklen_t peer_len = sizeof(struct sockaddr_in);
    quiche_conn *conn = quiche_connect(
        server_addr, scid, sizeof(scid), (struct sockaddr *)&local_storage,
        local_len, (struct sockaddr *)&peer, peer_len, config);
    if (!conn) {
        fprintf(stderr, "Failed to create connection\n");
        quiche_config_free(config);
        close(client_sock);
        return 1;
    }

    printf("Ping client started\n");
    printf("Press Ctrl-C to trigger connection migration\n");
    print_local_addr(client_sock);

    uint8_t out[MAX_DATAGRAM_SIZE], buf[65535];
    
    send_pending_packets(conn, &peer, &peer_len, out);

    struct timeval last_ping = {0}, ping_sent_time = {0};
    uint64_t stream_id = 0;
    int ping_seq = 0;
    bool waiting_for_pong = false;
    int packets_on_new_path = 0;

    while (!quiche_conn_is_closed(conn)) {
        struct timeval now;
        gettimeofday(&now, NULL);
        
        // Check migration status (timeout, etc.)
        check_migration_status(conn, &local_storage, local_len, &peer, peer_len, out);
        
        // 1. Determine maximum time to wait (based on QUIC timer)
        uint64_t quic_timeout_ns = quiche_conn_timeout_as_nanos(conn);
        uint64_t timeout_us = 1000;

        if (quic_timeout_ns > 0) {
            timeout_us = quic_timeout_ns / 1000;
            if (timeout_us == 0) timeout_us = 1; 
        } 
        
        // 2. Determine time until next PING is due
        if (quiche_conn_is_established(conn) && !waiting_for_pong) {
            long last_ping_us = (now.tv_sec - last_ping.tv_sec) * 1000000 +
                                (now.tv_usec - last_ping.tv_usec);
            
            long remaining_us = 1000000 - last_ping_us;

            if (remaining_us > 0) {
                if ((uint64_t)remaining_us < timeout_us) {
                    timeout_us = (uint64_t)remaining_us;
                }
            } else {
                timeout_us = 0;
            }
        }
        
        // 3. Setup file descriptors for select()
        fd_set readfds;
        FD_ZERO(&readfds);
        FD_SET(client_sock, &readfds);
        
        int max_fd = client_sock;
        
        // During migration, also monitor the old socket
        if (migration_in_progress && old_migration_sock >= 0) {
            FD_SET(old_migration_sock, &readfds);
            if (old_migration_sock > max_fd) {
                max_fd = old_migration_sock;
            }
        }

        struct timeval timeout_tv;
        timeout_tv.tv_sec = timeout_us / 1000000;
        timeout_tv.tv_usec = timeout_us % 1000000;
        
        int ret = select(max_fd + 1, &readfds, NULL, NULL, &timeout_tv);

        if (ret < 0) {
            if (errno != EINTR) {
                perror("select");
                break;
            }
        }
        
        gettimeofday(&now, NULL); 

        // 4. Handle Packet Reception on NEW socket
        if (ret > 0 && FD_ISSET(client_sock, &readfds)) {
            socklen_t current_peer_len = peer_len;
            ssize_t len = recvfrom(client_sock, buf, sizeof(buf), 0,
                                   (struct sockaddr *)&peer, &current_peer_len);

            if (len > 0) {
                peer_len = current_peer_len;

                quiche_recv_info ri = {.from = (struct sockaddr *)&peer,
                                       .from_len = peer_len,
                                       .to = (struct sockaddr *)&local_storage,
                                       .to_len = local_len};
                ssize_t recv_ret = quiche_conn_recv(conn, buf, len, &ri);
                
                if (recv_ret >= 0 || recv_ret == QUICHE_ERR_DONE) {
                    if (migration_in_progress) {
                        packets_on_new_path++;
                        printf("Received packet #%d on new socket (path validation in progress)\n", 
                               packets_on_new_path);
                        
                        // After receiving a few packets, path should be validated
                        // Try to complete migration
                        if (packets_on_new_path >= 2) {
                            complete_migration(conn, &local_storage, local_len, &peer, peer_len);
                            packets_on_new_path = 0;
                        }
                    }
                }

            } else if (len < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
                perror("recvfrom (new socket)");
            }
        }
        
        // 5. Handle Packet Reception on OLD socket (during migration)
        if (ret > 0 && migration_in_progress && old_migration_sock >= 0 && 
            FD_ISSET(old_migration_sock, &readfds)) {
            
            socklen_t current_peer_len = peer_len;
            ssize_t len = recvfrom(old_migration_sock, buf, sizeof(buf), 0,
                                   (struct sockaddr *)&peer, &current_peer_len);

            if (len > 0) {
                peer_len = current_peer_len;
                
                printf("Received packet on OLD socket during migration\n");

                // We need to temporarily switch back to receive on old socket
                // Save current socket
                int temp_sock = client_sock;
                client_sock = old_migration_sock;
                
                quiche_recv_info ri = {.from = (struct sockaddr *)&peer,
                                       .from_len = peer_len,
                                       .to = (struct sockaddr *)&local_storage,
                                       .to_len = local_len};
                quiche_conn_recv(conn, buf, len, &ri);
                
                // Restore new socket
                client_sock = temp_sock;
                
                // Send any pending packets
                send_pending_packets(conn, &peer, &peer_len, out);

            } else if (len < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
                perror("recvfrom (old socket)");
            }
        }
        
        // 6. Handle QUIC Timeout
        if (quiche_conn_timeout_as_nanos(conn) == 0) {
             quiche_conn_on_timeout(conn); 
             if (quiche_conn_is_closed(conn)) {
                 break; 
             }
        }
        
        // 7. Handle Migration Request (Ctrl-C)
        if (ip_change_requested) {
            ip_change_requested = false;
            
            if (!quiche_conn_is_established(conn)) {
                fprintf(stderr, "Cannot migrate: Connection not yet established.\n");
            } else if (migration_in_progress) {
                fprintf(stderr, "Migration already in progress, please wait...\n");
            } else {
                if (quiche_conn_available_dcids(conn) > 0) {
                    printf("\n=== Simulating IP change via Ctrl-C ===\n");
                    start_migration_probe(conn, &local_storage, &local_len, &peer,
                                         out, &peer_len);
                } else {
                    quiche_conn_send_ack_eliciting(conn);
                    send_pending_packets(conn, &peer, &peer_len, out);
                    printf("DCID pool empty (%lu available). Requesting more CIDs from server...\n",
                           quiche_conn_available_dcids(conn));
                    ip_change_requested = true;
                }
            }
        }

        // 8. Ping logic
        if (quiche_conn_is_established(conn)) {
            
            if (!waiting_for_pong &&
                (now.tv_sec - last_ping.tv_sec >= 1 || last_ping.tv_sec == 0)) {
                
                if (last_ping.tv_sec == 0) {
                    last_ping = now;
                }

                char ping_msg[64];
                snprintf(ping_msg, sizeof(ping_msg), "PING %d", ping_seq);
                stream_id = 0;
                
                ssize_t sent_ping = quiche_conn_stream_send(conn, stream_id, (uint8_t *)ping_msg,
                                                            strlen(ping_msg), false, NULL);

                if (sent_ping < 0 && sent_ping != QUICHE_ERR_DONE) {
                    fprintf(stderr, "Stream send error: %ld\n", sent_ping);
                } else if (sent_ping >= 0) {
                    gettimeofday(&ping_sent_time, NULL);
                    printf("Sent: %s\n", ping_msg);
                    
                    last_ping = now; 
                    waiting_for_pong = true;
                    ping_seq++;
                }
            }

            if (waiting_for_pong) {
                
                long rtt_us = (now.tv_sec - ping_sent_time.tv_sec) * 1000000 +
                              (now.tv_usec - ping_sent_time.tv_usec);

                if (rtt_us > 2000000) {
                    printf("Pong timeout detected\n");
                    waiting_for_pong = false;
                } else {
                    uint8_t data[4096];
                    bool fin;
                    ssize_t n = quiche_conn_stream_recv(conn, stream_id, data,
                                                        sizeof(data), &fin, NULL);
                    if (n > 0) {
                        printf("Received: %.*s (RTT: %.2f ms)\n", (int)n, data,
                               rtt_us / 1000.0);
                        waiting_for_pong = false;
                        last_ping = now;
                    }
                }
            }
        }

        // 9. Send any pending packets
        send_pending_packets(conn, &peer, &peer_len, out);
    }

    printf("Connection closed\n");
    
    // Cleanup
    if (old_migration_sock >= 0) {
        close(old_migration_sock);
    }
    quiche_conn_free(conn);
    quiche_config_free(config);
    close(client_sock);
    return 0;
}

int main(int argc, char *argv[]) { 
    return run_client(argc, argv); 
}

const char *quiche_error_msg(int err) {
    switch(err) {
        case QUICHE_ERR_DONE: return "Done";
        case QUICHE_ERR_BUFFER_TOO_SHORT: return "Buffer too short";
        case QUICHE_ERR_UNKNOWN_VERSION: return "Unknown version";
        case QUICHE_ERR_INVALID_FRAME: return "Invalid frame";
        case QUICHE_ERR_INVALID_PACKET: return "Invalid packet";
        case QUICHE_ERR_INVALID_STATE: return "Invalid state";
        case QUICHE_ERR_FLOW_CONTROL: return "Flow control";
        case QUICHE_ERR_STREAM_LIMIT: return "Stream limit";
        case QUICHE_ERR_INVALID_STREAM_STATE: return "Invalid stream state";
        case QUICHE_ERR_TLS_FAIL: return "TLS fail";
        case QUICHE_ERR_ID_LIMIT: return "ID Limit";
        case QUICHE_ERR_CRYPTO_BUFFER_EXCEEDED: return "Crypto buffer exceeded";
        case QUICHE_ERR_KEY_UPDATE: return "Key update error";
        case -18: return "Path validation failure";
        default: return "Unknown QUICHE error";
    }
}