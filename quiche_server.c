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

int main() {
    srand(time(NULL));
    
    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (!config) {
        fprintf(stderr, "Failed to create config\n");
        return 1;
    }
    
    if (quiche_config_load_cert_chain_from_pem_file(config, "cert.pem") != 0) {
        fprintf(stderr, "Failed to load cert\n");
        return 1;
    }
    if (quiche_config_load_priv_key_from_pem_file(config, "key.pem") != 0) {
        fprintf(stderr, "Failed to load key\n");
        return 1;
    }
    
    // ADD ALPN - use http/0.9 which is simple
    quiche_config_set_application_protos(config,
        (uint8_t *)"\x08http/0.9", 9);
    
    quiche_config_set_max_idle_timeout(config, 5000);
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);
    quiche_config_set_initial_max_streams_uni(config, 100);

    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) {
        perror("socket");
        return 1;
    }
    fcntl(sock, F_SETFL, O_NONBLOCK);

    struct sockaddr_in addr = {0};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(4433);
    addr.sin_addr.s_addr = INADDR_ANY;
    
    if (bind(sock, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        perror("bind");
        return 1;
    }

    printf("Server listening on port 4433\n");

    quiche_conn *conn = NULL;
    uint8_t out[MAX_DATAGRAM_SIZE];
    uint8_t buf[65535];
    struct sockaddr_storage peer;
    socklen_t peer_len;

    while (1) {
        peer_len = sizeof(peer);
        ssize_t len = recvfrom(sock, buf, sizeof(buf), 0, 
                              (struct sockaddr *)&peer, &peer_len);
        
        if (len < 0) {
            if (errno != EAGAIN && errno != EWOULDBLOCK) {
                perror("recvfrom");
            }
            usleep(1000);
            continue;
        }

        printf("Received %zd bytes\n", len);

        uint8_t type, scid[QUICHE_MAX_CONN_ID_LEN], dcid[QUICHE_MAX_CONN_ID_LEN];
        uint8_t token[256];
        size_t scid_len = sizeof(scid), dcid_len = sizeof(dcid), token_len = sizeof(token);
        uint32_t version;

        if (quiche_header_info(buf, len, LOCAL_CONN_ID_LEN, &version, &type,
                              scid, &scid_len, dcid, &dcid_len, 
                              token, &token_len) < 0) {
            fprintf(stderr, "Failed to parse header\n");
            continue;
        }

        if (!conn && quiche_version_is_supported(version)) {
            uint8_t cid[LOCAL_CONN_ID_LEN];
            for (int i = 0; i < LOCAL_CONN_ID_LEN; i++) cid[i] = rand() % 256;

            conn = quiche_accept(cid, LOCAL_CONN_ID_LEN, dcid, dcid_len,
                               (struct sockaddr *)&addr, sizeof(addr),
                               (struct sockaddr *)&peer, peer_len, config);
            
            if (!conn) {
                fprintf(stderr, "Failed to create connection\n");
                continue;
            }
            
            printf("Connection created\n");
        }

        if (conn) {
            quiche_recv_info ri = {
                (struct sockaddr *)&peer, peer_len,
                (struct sockaddr *)&addr, sizeof(addr)
            };

            ssize_t done = quiche_conn_recv(conn, buf, len, &ri);
            if (done < 0) {
                fprintf(stderr, "quiche_conn_recv failed: %zd\n", done);
            }

            if (quiche_conn_is_established(conn)) {
                static bool printed = false;
                if (!printed) {
                    printf("Connection established!\n");
                    printed = true;
                }
                
                quiche_stream_iter *r = quiche_conn_readable(conn);
                uint64_t s;
                
                while (quiche_stream_iter_next(r, &s)) {
                    uint8_t data[4096];
                    bool fin;
                    ssize_t n = quiche_conn_stream_recv(conn, s, data, sizeof(data), &fin, NULL);
                    
                    if (n > 0) {
                        printf("RX: ");
                        fwrite(data, 1, n, stdout);
                        printf("\n");
                        
                        quiche_conn_stream_send(conn, s, (uint8_t *)"ECHO: ", 6, false, NULL);
                        quiche_conn_stream_send(conn, s, data, n, true, NULL);
                        printf("Sent echo response\n");
                    }
                }
                quiche_stream_iter_free(r);
            }

            quiche_send_info si;
            ssize_t sent;
            while ((sent = quiche_conn_send(conn, out, sizeof(out), &si)) > 0) {
                sendto(sock, out, sent, 0, (struct sockaddr *)&peer, peer_len);
            }
            
            if (quiche_conn_is_closed(conn)) {
                printf("Connection closed\n");
                quiche_conn_free(conn);
                conn = NULL;
            }
        }
    }
}