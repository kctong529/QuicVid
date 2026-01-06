use quinn::{ClientConfig, Endpoint};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use rustls::client::danger::{ServerCertVerifier, HandshakeSignatureValid, ServerCertVerified};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::fs::File;
use std::io::Read;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider().install_default()
        .expect("Failed to install crypto provider");

    let mut rustls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    
    rustls_config.alpn_protocols = vec![b"hq-29".to_vec()];

    let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(rustls_config)?;

    // Bind to 0.0.0.0:0 so the OS chooses the best interface (h2-eth0)
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    let mut client_config = ClientConfig::new(Arc::new(quic_config));

    // Allow the client to send datagrams
    let mut transport = quinn::TransportConfig::default();
    transport.datagram_receive_buffer_size(Some(64 * 1024)); 
    client_config.transport_config(Arc::new(transport));

    endpoint.set_default_client_config(client_config);
    
    let remote = "10.0.0.1:4433".parse()?; // Target h1
    println!("Connecting to h1 ({}) from h2...", remote);
    
    let conn = endpoint.connect(remote, "localhost")?.await?;

    let mut mouse_dev = File::open("/dev/input/mice").expect("Could not open mouse device - try running with sudo");
    let mut mouse_data = [0u8; 3]; // Standard PS/2 mouse packet is 3 bytes
    
    let mut cur_x: i32 = 0;
    let mut cur_y: i32 = 0;
    println!("Streaming REAL mouse data. Move your mouse!");

    for i in 1..2000 {
        // 60Hz update rate for smooth tracking
        sleep(Duration::from_millis(16)).await;

        // Read 3 bytes from the mouse device
        mouse_dev.read_exact(&mut mouse_data)?;

        // Byte 1: Buttons and signs
        // Byte 2: Relative X movement
        // Byte 3: Relative Y movement
        let dx = mouse_data[1] as i8 as i32;
        let dy = mouse_data[2] as i8 as i32;

        cur_x += dx;
        cur_y -= dy; // Invert Y because screen coordinates go down

        // Clamp coordinates to keep the visualizer from breaking
        cur_x = cur_x.clamp(-1000, 1000);
        cur_y = cur_y.clamp(-1000, 1000);

        // Pack data into 8 bytes: [x: i32 (4b)][y: i32 (4b)]
        let mut packet = [0u8; 8];
        packet[0..4].copy_from_slice(&cur_x.to_be_bytes());
        packet[4..8].copy_from_slice(&cur_y.to_be_bytes());
        
        // Migrate every 100 packets (~1.6 seconds)
        if i % 100 == 0 {
            println!("\n--- MIGRATING PORT ---");
            let new_socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
            new_socket.set_nonblocking(true)?;
            endpoint.rebind(new_socket)?;
        }

        // Sending the datagram directly via the connection
        conn.send_datagram(packet.to_vec().into())?;

        let stats = conn.stats();
        println!(
            "Pkt {:03} | Pos: ({:>4}, {:>4}) | RTT: {:?}",
            i, cur_x, cur_y, stats.path.rtt
        );
    }

    Ok(())
}

#[derive(Debug)]
struct SkipServerVerification;

impl ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        // Fix for E0107: Removed <'_>
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        // Fix for E0107: Removed <'_>
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ED25519,
        ]
    }
}