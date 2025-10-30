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

int main() {
    srand(time(NULL));

    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (!config) {
        fprintf(stderr, "Failed to create config\n");
        return 1;
    }
    
    // ADD MATCHING ALPN
    quiche_config_set_application_protos(config,
        (uint8_t *)"\x08http/0.9", 9);
    
    quiche_config_verify_peer(config, false);
    quiche_config_set_max_idle_timeout(config, 5000);
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);

    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) {
        perror("socket");
        return 1;
    }
    fcntl(sock, F_SETFL, O_NONBLOCK);

    struct sockaddr_in local = {0}, peer = {0};
    local.sin_family = AF_INET;
    local.sin_addr.s_addr = INADDR_ANY;
    
    if (bind(sock, (struct sockaddr *)&local, sizeof(local)) < 0) {
        perror("bind");
        return 1;
    }

    socklen_t local_len = sizeof(local);
    getsockname(sock, (struct sockaddr *)&local, &local_len);

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

    printf("Client started\n");

    uint8_t out[MAX_DATAGRAM_SIZE], buf[65535];
    quiche_send_info si;
    ssize_t sent;

    while ((sent = quiche_conn_send(conn, out, sizeof(out), &si)) > 0) {
        ssize_t done = sendto(sock, out, sent, 0, (struct sockaddr *)&peer, sizeof(peer));
        printf("Sent %zd bytes\n", done);
    }

    bool msg_sent = false, got_reply = false;

    for (int i = 0; i < 5000 && !got_reply; i++) {
        ssize_t len = recvfrom(sock, buf, sizeof(buf), 0, NULL, NULL);
        
        if (len > 0) {
            printf("Received %zd bytes\n", len);
            
            quiche_recv_info ri = {
                (struct sockaddr *)&peer, sizeof(peer),
                (struct sockaddr *)&local, local_len
            };
            
            quiche_conn_recv(conn, buf, len, &ri);
        }

        if (quiche_conn_is_established(conn) && !msg_sent) {
            printf("Connection established! Sending message...\n");
            quiche_conn_stream_send(conn, 0, (uint8_t *)"Hello", 5, true, NULL);
            msg_sent = true;
        }

        if (msg_sent) {
            uint8_t data[4096];
            bool fin;
            ssize_t n = quiche_conn_stream_recv(conn, 0, data, sizeof(data), &fin, NULL);
            if (n > 0) {
                printf("Reply: ");
                fwrite(data, 1, n, stdout);
                printf("\n");
                if (fin) got_reply = true;
            }
        }

        while ((sent = quiche_conn_send(conn, out, sizeof(out), &si)) > 0) {
            sendto(sock, out, sent, 0, (struct sockaddr *)&peer, sizeof(peer));
        }

        if (quiche_conn_is_closed(conn)) {
            printf("Connection closed\n");
            break;
        }

        usleep(1000);
    }

    quiche_conn_free(conn);
    quiche_config_free(config);
    close(sock);
    
    return 0;
}