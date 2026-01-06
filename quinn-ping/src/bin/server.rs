use quinn::{Endpoint, ServerConfig};
use std::{error::Error, net::SocketAddr, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider().install_default()
        .expect("Failed to install crypto provider");

    let (server_config, _) = configure_server()?;
    let addr = "0.0.0.0:4433".parse()?; // Listens on all interfaces
    let endpoint = Endpoint::server(server_config, addr)?;
    println!("Server listening on 0.0.0.0:4433 (Mininet h1)");

    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(async move {
            let connection = conn.await.unwrap();

            let (mut send, mut recv) = connection.accept_bi().await.unwrap();
            
            // Generate a short ID for cleaner logs
            let stable_id = connection.stable_id();
            println!("[Session {:?}] Client connected from {}", stable_id, connection.remote_address());

            let mut buf = [0u8; 4];
            loop {
                if recv.read_exact(&mut buf).await.is_err() { 
                    println!("[Session {:?}] Client disconnected.", stable_id);
                    break; 
                }
                
                println!(
                    "REQ: {} | CID: {:?} | FROM: {} | RTT: {:?}", 
                    String::from_utf8_lossy(&buf), 
                    stable_id,
                    connection.remote_address(),
                    connection.stats().path.rtt // The server's view of RTT
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
    
    server_crypto.alpn_protocols = vec![b"hq-29".to_vec()];

    let server_config = ServerConfig::with_crypto(Arc::new(quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?));
    
    Ok((server_config, cert_der))
}