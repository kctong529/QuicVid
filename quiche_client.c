#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <quiche.h>
#include <time.h>

#define LOCAL_CONN_ID_LEN 16
#define MAX_DATAGRAM_SIZE 1350

static void generate_cid(uint8_t *buf, size_t len) {
    for (size_t i = 0; i < len; i++) {
        buf[i] = rand() % 256;
    }
}

int main(int argc, char *argv[]) {
    srand(time(NULL));
    
    // Initialize configurations
    quiche_config *config = quiche_config_new(QUICHE_PROTOCOL_VERSION);
    if (config == NULL) {
        fprintf(stderr, "Failed to create config\n");
        return 1;
    }
    
    // Set application protocols for HTTP/3
    quiche_config_set_application_protos(config, 
        (uint8_t *)"\x02h3\x08http/0.9", 12);
    
    // Configure verification (disable for testing with self-signed certs)
    quiche_config_verify_peer(config, false);
    
    // Set maximum idle timeout (ms)
    quiche_config_set_max_idle_timeout(config, 5000);
    
    // Set initial max data
    quiche_config_set_initial_max_data(config, 10000000);
    quiche_config_set_initial_max_stream_data_bidi_local(config, 1000000);
    quiche_config_set_initial_max_stream_data_bidi_remote(config, 1000000);
    quiche_config_set_initial_max_streams_bidi(config, 100);
    
    // Generate source connection ID
    uint8_t scid[LOCAL_CONN_ID_LEN];
    generate_cid(scid, sizeof(scid));
    
    // Generate destination connection ID (for initial packet)
    uint8_t dcid[LOCAL_CONN_ID_LEN];
    generate_cid(dcid, sizeof(dcid));
    
    // Create socket address for server
    struct sockaddr_in local_addr;
    local_addr.sin_family = AF_INET;
    local_addr.sin_port = 0;  // Let OS pick a port
    local_addr.sin_addr.s_addr = INADDR_ANY;
    
    struct sockaddr_in peer_addr;
    peer_addr.sin_family = AF_INET;
    peer_addr.sin_port = htons(4433);
    inet_pton(AF_INET, "127.0.0.1", &peer_addr.sin_addr);
    
    // Create connection with correct number of arguments
    // quiche_connect needs: server_name, scid, scid_len, 
    // local_addr, local_len, peer_addr, peer_len, config
    quiche_conn *conn = quiche_connect(
        "localhost",                           // server name for SNI
        scid, sizeof(scid),                   // source connection ID
        (struct sockaddr *)&local_addr, sizeof(local_addr),  // local address
        (struct sockaddr *)&peer_addr, sizeof(peer_addr),    // peer address
        config                                 // configuration
    );
    
    if (conn == NULL) {
        fprintf(stderr, "Failed to create connection\n");
        quiche_config_free(config);
        return 1;
    }
    
    printf("Connection created successfully!\n");
    printf("Is established: %s\n", 
           quiche_conn_is_established(conn) ? "true" : "false");
    
    // Create UDP socket
    int sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (sock < 0) {
        perror("Failed to create socket");
        quiche_conn_free(conn);
        quiche_config_free(config);
        return 1;
    }
    
    // Bind the socket
    if (bind(sock, (struct sockaddr *)&local_addr, sizeof(local_addr)) < 0) {
        perror("Failed to bind socket");
        close(sock);
        quiche_conn_free(conn);
        quiche_config_free(config);
        return 1;
    }
    
    // Prepare initial packet
    uint8_t out[MAX_DATAGRAM_SIZE];
    
    // Create send_info structure for sending
    quiche_send_info send_info;
    
    // Write initial packet - now requires send_info parameter
    ssize_t written = quiche_conn_send(conn, out, sizeof(out), &send_info);
    
    if (written == QUICHE_ERR_DONE) {
        fprintf(stderr, "No packets to send\n");
        close(sock);
        quiche_conn_free(conn);
        quiche_config_free(config);
        return 1;
    }
    
    if (written < 0) {
        fprintf(stderr, "Failed to create initial packet: %zd\n", written);
        close(sock);
        quiche_conn_free(conn);
        quiche_config_free(config);
        return 1;
    }
    
    printf("Sending initial packet (%zd bytes) to %s:%d\n", 
           written,
           inet_ntoa(((struct sockaddr_in*)&send_info.to)->sin_addr),
           ntohs(((struct sockaddr_in*)&send_info.to)->sin_port));
    
    // Send packet to the address specified in send_info
    ssize_t sent = sendto(sock, out, written, 0, 
                          (struct sockaddr *)&send_info.to, 
                          send_info.to_len);
    
    if (sent != written) {
        perror("Failed to send");
        close(sock);
        quiche_conn_free(conn);
        quiche_config_free(config);
        return 1;
    }
    
    printf("Initial packet sent successfully!\n");
    
    // Event loop for receiving packets
    int handshake_completed = 0;
    while (!quiche_conn_is_closed(conn)) {
        uint8_t buf[65535];
        struct sockaddr_storage from;
        socklen_t from_len = sizeof(from);
        
        // Receive packet with timeout
        fd_set readfds;
        FD_ZERO(&readfds);
        FD_SET(sock, &readfds);
        
        struct timeval tv;
        tv.tv_sec = 1;
        tv.tv_usec = 0;
        
        int result = select(sock + 1, &readfds, NULL, NULL, &tv);
        
        if (result > 0) {
            ssize_t recvd = recvfrom(sock, buf, sizeof(buf), 0,
                                     (struct sockaddr *)&from, &from_len);
            
            if (recvd < 0) {
                perror("recv failed");
                break;
            }
            
            printf("Received %zd bytes\n", recvd);
            
            // Create recv_info structure for receiving
            quiche_recv_info recv_info = {
                .from = (struct sockaddr *)&from,
                .from_len = from_len,
                .to = (struct sockaddr *)&local_addr,
                .to_len = sizeof(local_addr)
            };
            
            // Process received packet - now requires recv_info
            ssize_t processed = quiche_conn_recv(conn, buf, recvd, &recv_info);
            
            if (processed < 0) {
                fprintf(stderr, "Failed to process packet: %zd\n", processed);
                continue;
            }
            
            printf("Processed %zd bytes\n", processed);
        }
        
        // Check if connection is established
        if (quiche_conn_is_established(conn) && !handshake_completed) {
            printf("*** Connection established! ***\n");
            
            const uint8_t *app_proto;
            size_t app_proto_len;
            quiche_conn_application_proto(conn, &app_proto, &app_proto_len);
            
            printf("Application protocol: %.*s\n", 
                   (int)app_proto_len, app_proto);
            
            handshake_completed = 1;
            
            // Could send HTTP request here if using HTTP/3
            // For now, we'll close after establishing connection
            quiche_conn_close(conn, true, 0, NULL, 0);
        }
        
        // Send any pending packets
        while (1) {
            quiche_send_info send_info;
            ssize_t written = quiche_conn_send(conn, out, sizeof(out), &send_info);
            
            if (written == QUICHE_ERR_DONE) {
                break;
            }
            
            if (written < 0) {
                fprintf(stderr, "Failed to write packet: %zd\n", written);
                break;
            }
            
            ssize_t sent = sendto(sock, out, written, 0,
                                  (struct sockaddr *)&send_info.to,
                                  send_info.to_len);
            
            if (sent != written) {
                perror("Failed to send");
                break;
            }
            
            printf("Sent %zd bytes\n", sent);
        }
        
        // Exit after handshake for this demo
        if (handshake_completed && quiche_conn_is_closed(conn)) {
            printf("Connection closed\n");
            break;
        }
    }
    
    // Get connection stats
    quiche_stats stats;
    quiche_path_stats path_stats;

    quiche_conn_stats(conn, &stats);
    quiche_conn_path_stats(conn, 0, &path_stats);

    printf("\nConnection Statistics:\n");
    printf("  Packets sent: %zu\n", stats.sent);
    printf("  Packets received: %zu\n", stats.recv);
    printf("  Packets lost: %zu\n", stats.lost);
    printf("  RTT: %llu ms\n", path_stats.rtt);
    
    // Cleanup
    close(sock);
    quiche_conn_free(conn);
    quiche_config_free(config);
    
    return 0;
}