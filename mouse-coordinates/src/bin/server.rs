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
            println!("Tracking client: {:?}", connection.stable_id());

            let mut buf = [0u8; 8]; // Buffer for x and y

            loop {
                if recv.read_exact(&mut buf).await.is_err() { break; }
                
                // Decode coordinates
                let x = i32::from_be_bytes(buf[0..4].try_into().unwrap());
                let y = i32::from_be_bytes(buf[4..8].try_into().unwrap());

                println!(
                    "ID: {:?} | MOUSE: X:{:>4} Y:{:>4} | FROM: {}", 
                    connection.stable_id(), x, y, connection.remote_address()
                );

                // Send 1-byte ACK
                send.write_all(&[1]).await.unwrap();
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