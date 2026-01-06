use quinn::{ClientConfig, Endpoint};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use rustls::client::danger::{ServerCertVerifier, HandshakeSignatureValid, ServerCertVerified};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize Crypto Provider
    rustls::crypto::ring::default_provider().install_default()
        .expect("Failed to install crypto provider");

    // 2. Build the Rustls config
    let mut rustls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_no_client_auth();
    
    // IMPORTANT: QUIC requires ALPN to be set
    rustls_config.alpn_protocols = vec![b"hq-29".to_vec()]; // Standard for HTTP 0.9/Testing

    // 3. Wrap it for Quinn (This fixes error E0277)
    let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(rustls_config)?;
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(ClientConfig::new(Arc::new(quic_config)));

    let remote = "127.0.0.1:4433".parse()?;
    println!("Connecting to {}...", remote);
    
    let conn = endpoint.connect(remote, "localhost")?.await?;
    let (mut send, mut recv) = conn.open_bi().await?;

    let mut buf = [0u8; 4];
    for i in 1..20 {
        sleep(Duration::from_secs(1)).await;
        
        if i % 5 == 0 {
            println!("\n--- MIGRATING TO NEW PORT ---");
            let new_socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
            new_socket.set_nonblocking(true)?;
            endpoint.rebind(new_socket)?;
        }

        send.write_all(b"PING").await?;
        recv.read_exact(&mut buf).await?;
        println!("Round {}: Received {} from server", i, String::from_utf8_lossy(&buf));
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