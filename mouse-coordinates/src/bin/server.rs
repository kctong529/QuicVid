use quinn::{Endpoint, ServerConfig};
use std::{error::Error, sync::Arc};
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider().install_default().ok();

    let (server_config, _) = configure_server()?;
    let addr = "0.0.0.0:4433".parse()?; 
    let endpoint = Endpoint::server(server_config, addr)?;
    println!("Server listening on 0.0.0.0:4433 (Mininet h1)");

    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(async move {
            let connection = conn.await.unwrap();
            println!("[Session {:?}] Client connected from {}", connection.stable_id(), connection.remote_address());

            loop {
                // Receive the raw datagram (Low latency, unreliable)
                match connection.read_datagram().await {
                    Ok(bytes) => {
                        if bytes.len() < 8 { continue; }
                        let x = i32::from_be_bytes(bytes[0..4].try_into().unwrap());
                        let y = i32::from_be_bytes(bytes[4..8].try_into().unwrap());

                        println!(
                            "ID: {:?} | MOUSE: X:{:>4} Y:{:>4} | FROM: {} | RTT: {:?}", 
                            connection.stable_id(), x, y, 
                            connection.remote_address(),
                            connection.stats().path.rtt
                        );
                    }
                    Err(_) => {
                        println!("Connection closed.");
                        break;
                    }
                }
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

    let mut transport_config = quinn::TransportConfig::default();
    transport_config.datagram_receive_buffer_size(Some(64 * 1024)); // 64 KB buffer

    let mut server_config = ServerConfig::with_crypto(Arc::new(quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)?));
    server_config.transport_config(Arc::new(transport_config));
    
    Ok((server_config, cert_der))
}