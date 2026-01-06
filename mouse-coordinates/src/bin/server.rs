use quinn::{Endpoint, ServerConfig};
use std::{error::Error, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider().install_default().ok();

    let (server_config, _) = configure_server()?;
    let addr = "0.0.0.0:4433".parse()?; 
    let endpoint = Endpoint::server(server_config, addr)?;

    // Clear screen and hide cursor
    print!("{}[2J{}[?25l", 27 as char, 27 as char);

    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(async move {
            let connection = conn.await.unwrap();

            loop {
                match connection.read_datagram().await {
                    Ok(bytes) => {
                        if bytes.len() < 8 { continue; }
                        let x = i32::from_be_bytes(bytes[0..4].try_into().unwrap());
                        let y = i32::from_be_bytes(bytes[4..8].try_into().unwrap());
                        
                        // Normalize coordinates to terminal size (roughly 80x24)
                        // Assuming input is -100 to 100, map to terminal center
                        let term_x = (x / 4) + 40;
                        let term_y = (y / 8) + 12;

                        // 1. Clear previous status line and dot (optional: clear whole screen for smoothness)
                        // 2. Move to new X/Y
                        // 3. Print a cursor-like character
                        print!("{}[2J{}[{};{}Hcursor -> ↖", 27 as char, 27 as char, term_y, term_x);

                        // Status bar at the bottom
                        print!("{}[24;1H{}[2KPath: {} | RTT: {:?}", 
                            27 as char, 27 as char,
                            connection.remote_address(), 
                            connection.stats().path.rtt
                        );
                        
                        use std::io::{self, Write};
                        io::stdout().flush().unwrap();
                    }
                    Err(_) => break,
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