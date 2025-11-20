// quiche_client.c

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

#define MAX_DATAGRAM_SIZE 1350

atomic_bool ip_change_requested = false;
int client_sock = -1;

void sigint_handler(int sig) {
    ip_change_requested = true;
}

void perform_ip_change_migration(quiche_conn* conn) {
    printf("Simulating local IP change via Ctrl-C…\n");

    // Create a new UDP socket with a new ephemeral port
    int new_sock = socket(AF_INET, SOCK_DGRAM, 0);

    struct sockaddr_in new_local;
    memset(&new_local, 0, sizeof(new_local));
    new_local.sin_family = AF_INET;
    new_local.sin_addr.s_addr = htonl(INADDR_ANY);
    new_local.sin_port = 0; // let OS pick a new port — simulates "new IP:port"

    bind(new_sock, (struct sockaddr*)&new_local, sizeof(new_local));

    // Tell quiche about the new source address
    socklen_t sl = sizeof(new_local);

    uint64_t seq;
    int rc = quiche_conn_migrate_source(conn,
        (struct sockaddr*)&new_local, sl, &seq);

    if (rc != 0) {
        fprintf(stderr, "migrate_source failed: %d\n", rc);
        return;
    }

    printf("Migration requested (seq=%llu). Now sending on the new socket.\n",
           (unsigned long long)seq);

    // Update client state to use new_sock
    client_sock = new_sock;
}

int main() {
    srand(time(NULL));
    signal(SIGINT, sigint_handler);

    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (!config) {
        fprintf(stderr, "Failed to create config\n");
        return 1;
    }
    
    quiche_config_set_application_protos(config,
        (uint8_t *)"\x08http/0.9", 9);
    
    quiche_config_verify_peer(config, false);
    quiche_config_set_max_idle_timeout(config, 30000);
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);

    client_sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (client_sock < 0) {
        perror("socket");
        return 1;
    }
    fcntl(client_sock, F_SETFL, O_NONBLOCK);

    struct sockaddr_in local = {0}, peer = {0};
    local.sin_family = AF_INET;
    local.sin_addr.s_addr = INADDR_ANY;
    
    if (bind(client_sock, (struct sockaddr *)&local, sizeof(local)) < 0) {
        perror("bind");
        return 1;
    }

    socklen_t local_len = sizeof(local);
    getsockname(client_sock, (struct sockaddr *)&local, &local_len);

    peer.sin_family = AF_INET;
    peer.sin_port = htons(4433);
    inet_pton(AF_INET, "127.0.0.1", &peer.sin_addr);

    uint8_t scid[QUICHE_MAX_CONN_ID_LEN];
    for (int i = 0; i < sizeof(scid); i++) scid[i] = rand() % 256;

    quiche_conn *conn = quiche_connect("localhost", scid, sizeof(scid),
                                      (struct sockaddr *)&local, local_len,
                                      (struct sockaddr *)&peer, sizeof(peer),
                                      config);
    if (!conn) {
        fprintf(stderr, "Failed to create connection\n");
        return 1;
    }

    printf("Ping client started\n");

    uint8_t out[MAX_DATAGRAM_SIZE], buf[65535];
    quiche_send_info si;
    ssize_t sent;

    // Send initial handshake packets
    while ((sent = quiche_conn_send(conn, out, sizeof(out), &si)) > 0) {
        sendto(client_sock, out, sent, 0, (struct sockaddr *)&peer, sizeof(peer));
        printf("Sent %zd bytes\n", sent);
    }

    struct timeval last_ping = {0};
    uint64_t stream_id = 0;
    int ping_seq = 0;
    struct timeval ping_sent_time = {0};
    bool waiting_for_pong = false;

    while (!quiche_conn_is_closed(conn)) {
        // Detect interrupt flag
        if (ip_change_requested) {
            ip_change_requested = false;
            perform_ip_change_migration(conn);
        }

        // Receive packets
        ssize_t len = recvfrom(client_sock, buf, sizeof(buf), 0, NULL, NULL);
        
        if (len > 0) {
            printf("Received %zd bytes\n", len);
            
            quiche_recv_info ri = {
                (struct sockaddr *)&peer, sizeof(peer),
                (struct sockaddr *)&local, local_len
            };
            
            ssize_t done = quiche_conn_recv(conn, buf, len, &ri);
            if (done < 0) {
                fprintf(stderr, "quiche_conn_recv failed: %zd\n", done);
            }
            
            // CRITICAL: Send response packets immediately after receiving
            while ((sent = quiche_conn_send(conn, out, sizeof(out), &si)) > 0) {
                sendto(client_sock, out, sent, 0, (struct sockaddr *)&peer, sizeof(peer));
                printf("Sent %zd bytes in response\n", sent);
            }
        } else if (len < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
            perror("recvfrom");
        }

        // Check if connection is established
        if (quiche_conn_is_established(conn)) {
            static bool printed = false;
            if (!printed) {
                printf("Connection established!\n");
                printed = true;
            }

            // Send ping every 1 second
            struct timeval now;
            gettimeofday(&now, NULL);
            
            if (!waiting_for_pong && 
                (now.tv_sec - last_ping.tv_sec >= 1 || last_ping.tv_sec == 0)) {
                
                char ping_msg[64];
                snprintf(ping_msg, sizeof(ping_msg), "PING %d", ping_seq);
                
                // Use same stream for all pings
                stream_id = 0;
                
                quiche_conn_stream_send(conn, stream_id, 
                                      (uint8_t *)ping_msg, strlen(ping_msg), 
                                      false, NULL);  // Don't fin the stream
                
                gettimeofday(&ping_sent_time, NULL);
                printf("Sent: %s\n", ping_msg);
                
                last_ping = now;
                waiting_for_pong = true;
                ping_seq++;
            }

            // Check for responses
            if (waiting_for_pong) {
                uint8_t data[4096];
                bool fin;
                ssize_t n = quiche_conn_stream_recv(conn, stream_id, data, sizeof(data), &fin, NULL);
                
                if (n > 0) {
                    struct timeval now;
                    gettimeofday(&now, NULL);
                    
                    long rtt_us = (now.tv_sec - ping_sent_time.tv_sec) * 1000000 +
                                 (now.tv_usec - ping_sent_time.tv_usec);
                    
                    printf("Received: ");
                    fwrite(data, 1, n, stdout);
                    printf(" (RTT: %.2f ms)\n", rtt_us / 1000.0);
                    
                    waiting_for_pong = false;
                }
            }
        }

        // Always try to send pending packets (ACKs, etc)
        while ((sent = quiche_conn_send(conn, out, sizeof(out), &si)) > 0) {
            sendto(client_sock, out, sent, 0, (struct sockaddr *)&peer, sizeof(peer));
        }

        usleep(1000);  // 1ms sleep
    }

    printf("Connection closed\n");

    quiche_conn_free(conn);
    quiche_config_free(config);
    close(client_sock);
    
    return 0;
}