use crate::{control, tls};
use quinn::Connection;
use std::net::SocketAddr;

pub async fn run(listen: SocketAddr) -> anyhow::Result<()> {
    let server_config = tls::server_config()?;
    let endpoint = quinn::Endpoint::server(server_config, listen)?;

    println!("event=server_started listen={}", endpoint.local_addr()?);

    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                match incoming {
                    Some(incoming) => {
                        tokio::spawn(async move {
                            match incoming.await {
                                Ok(connection) => {
                                    if let Err(error) = handle_connection(connection).await {
                                        eprintln!(
                                            "event=connection_error error={error:#}"
                                        );
                                    }
                                }
                                Err(error) => {
                                    eprintln!(
                                        "event=handshake_failed error={error}"
                                    );
                                }
                            }
                        });
                    }
                    None => break,
                }
            }

            result = tokio::signal::ctrl_c() => {
                result?;
                println!("event=server_shutdown_requested");
                break;
            }
        }
    }

    endpoint.close(0u32.into(), b"server shutdown");
    endpoint.wait_idle().await;

    println!("event=server_stopped");

    Ok(())
}

async fn handle_connection(connection: Connection) -> anyhow::Result<()> {
    println!(
        "event=client_connected connection={} peer={}",
        connection.stable_id(),
        connection.remote_address(),
    );

    let (mut send, mut recv) = connection.accept_bi().await?;

    let request = recv.read_to_end(1024).await?;
    let request = String::from_utf8(request)?;

    let session_id = control::parse_hello(&request)?;

    println!(
        "event=client_hello session={} connection={} peer={}",
        session_id,
        connection.stable_id(),
        connection.remote_address(),
    );

    let response = control::acknowledgement(session_id);

    send.write_all(response.as_bytes()).await?;
    send.finish()?;

    println!(
        "event=hello_acknowledged session={} connection={}",
        session_id,
        connection.stable_id(),
    );

    connection.closed().await;

    println!(
        "event=client_disconnected session={} connection={}",
        session_id,
        connection.stable_id(),
    );

    Ok(())
}
