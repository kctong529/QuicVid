use quinn::{Endpoint, ServerConfig};
use std::{error::Error, net::SocketAddr, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider().install_default()
        .expect("Failed to install crypto provider");

    let (server_config, _) = configure_server()?;
    let addr = "127.0.0.1:4433".parse()?;
    let endpoint = Endpoint::server(server_config, addr)?;
    println!("Server listening on {}", addr);

    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(async move {
            let connection = conn.await.unwrap();
            // Accept the bidirectional stream from the client
            let (mut send, mut recv) = connection.accept_bi().await.unwrap();
            println!("Client connected. Starting ping-pong loop...");

            let mut buf = [0u8; 4];
            loop {
                if recv.read_exact(&mut buf).await.is_err() { break; }
                println!("Received {} | CID: {:?} | From: {}", 
                    String::from_utf8_lossy(&buf), 
                    connection.stable_id(), // This ID stays constant
                    connection.remote_address() // This changes at Round 5
                );
                send.write_all(b"PONG").await.unwrap();
            }
        });
    }
    Ok(())
}

fn configure_server() -> Result<(ServerConfig, Vec<u8>), Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let cert_der = cert.cert.der().to_vec();
    let priv_key = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    
    let mut server_crypto = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone().into()], priv_key.into())?;
    
    // THIS IS THE FIX: The server must match the client's ALPN
    server_crypto.alpn_protocols = vec![b"hq-29".to_vec()];

    // Wrap for Quinn
    let server_config = ServerConfig::with_crypto(Arc::new(quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?));
    
    Ok((server_config, cert_der))
}