use crate::tls;
use quinn::Connection;
use std::net::SocketAddr;

pub async fn run(listen: SocketAddr) -> anyhow::Result<()> {
    let server_config = tls::server_config()?;
    let endpoint = quinn::Endpoint::server(server_config, listen)?;

    println!("event=server_started listen={}", endpoint.local_addr()?);

    while let Some(incoming) = endpoint.accept().await {
        tokio::spawn(async move {
            match incoming.await {
                Ok(connection) => {
                    if let Err(error) = handle_connection(connection).await {
                        eprintln!("event=connection_error error={error:#}");
                    }
                }
                Err(error) => {
                    eprintln!("event=handshake_failed error={error}");
                }
            }
        });
    }

    Ok(())
}

async fn handle_connection(connection: Connection) -> anyhow::Result<()> {
    println!(
        "event=client_connected connection={} peer={}",
        connection.stable_id(),
        connection.remote_address(),
    );

    connection.closed().await;

    println!(
        "event=client_disconnected connection={}",
        connection.stable_id(),
    );

    Ok(())
}
