use quinn::{Endpoint, ServerConfig};
use std::{error::Error, sync::Arc};
use std::io::Write;

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
            let mut last_pos: Option<(i32, i32)> = None;

            loop {
                match connection.read_datagram().await {
                    Ok(bytes) => {
                        if bytes.len() < 8 { continue; }
                        let x = i32::from_be_bytes(bytes[0..4].try_into().unwrap());
                        let y = i32::from_be_bytes(bytes[4..8].try_into().unwrap());

                        // Scale -1000..1000 to roughly 100x36 terminal space
                        // We map -1000..1000 to 1..100 (X) and 1..36 (Y)
                        let term_x = (x / 20) + 50;  // Center is 50
                        let term_y = (y / 55) + 18;  // Center is 18

                        // Ensure drawing stays within terminal bounds to prevent glitching
                        let safe_x = term_x.clamp(1, 100);
                        let safe_y = term_y.clamp(1, 36);

                        // Erase the PREVIOUS position
                        if let Some((old_y, old_x)) = last_pos {
                            print!("{}[{};{}H ", 27 as char, old_y, old_x);
                        }

                        // Draw current position
                        print!("{}[{};{}H↖", 27 as char, safe_y, safe_x);

                        // Update tracker
                        last_pos = Some((safe_y, safe_x));
                        
                        // Status line
                        print!("{}[37;1H{}[2KPos: {:>4},{:>4} | IP: {} | RTT: {:?}",
                            27 as char, 27 as char, x, y, 
                            connection.remote_address(),
                            connection.stats().path.rtt
                        );
                        std::io::stdout().flush().unwrap();
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