use quinn::{Endpoint, ServerConfig};
use std::{error::Error, sync::Arc, io::Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    rustls::crypto::ring::default_provider().install_default().ok();

    let (server_config, _) = configure_server()?;
    let addr = "0.0.0.0:4433".parse()?; 
    let endpoint = Endpoint::server(server_config, addr)?;

    // Clear screen and draw the 100x36 boundary box
    print!("\x1b[2J\x1b[H"); // Clear and Home
    draw_boundary(100, 36);

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

                        // CALIBRATED CENTERING:
                        // Map -500..500 to 2..99 (X) and 2..35 (Y) to stay inside border                    
                        let term_x = ((x + 500) * 97 / 1000) + 2;
                        let term_y = ((y + 500) * 33 / 1000) + 2;

                        // Ensure drawing stays within terminal bounds to prevent glitching
                        let safe_x = term_x.clamp(2, 99);
                        let safe_y = term_y.clamp(2, 35);

                        // Erase the PREVIOUS position
                        if let Some((old_y, old_x)) = last_pos {
                            print!("\x1b[{};{}H ", old_y, old_x);
                        }

                        print!("\x1b[{};{}H↖", safe_y, safe_x);
                        last_pos = Some((safe_y, safe_x));
                        
                        // Status line on Row 38 (outside the box)
                        print!("\x1b[38;1H\x1b[2K[QUIC ID: {:?}] IP: {} | RTT: {:?} | Pos: {},{}", 
                            connection.stable_id(),
                            connection.remote_address(), 
                            connection.stats().path.rtt, 
                            x, y
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

fn draw_boundary(width: i32, height: i32) {
    // Top border
    print!("\x1b[1;1H+");
    print!("{}", "-".repeat((width - 2) as usize));
    print!("+");
    // Sides
    for y in 2..height {
        print!("\x1b[{};1H|", y);
        print!("\x1b[{};{}H|", y, width);
    }
    // Bottom border
    print!("\x1b[{};1H+", height);
    print!("{}", "-".repeat((width - 2) as usize));
    print!("+");
    std::io::stdout().flush().unwrap();
}