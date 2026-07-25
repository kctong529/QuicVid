use crate::{control, media::MediaDatagram, tls};
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

    // Initial QuicVid control handshake.
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

    loop {
        match connection.read_datagram().await {
            Ok(bytes) => {
                let media = match MediaDatagram::decode(&bytes) {
                    Ok(media) => media,

                    Err(error) => {
                        eprintln!(
                            "event=media_datagram_invalid connection={} error={error:#}",
                            connection.stable_id(),
                        );

                        continue;
                    }
                };

                if media.session_id != session_id {
                    eprintln!(
                        "event=media_session_mismatch expected={} got={} frame={}",
                        session_id, media.session_id, media.frame_id,
                    );

                    continue;
                }

                if media.chunk_index != 0 || media.chunk_count != 1 {
                    eprintln!(
                        "event=fake_frame_chunk_invalid session={} frame={} chunk={}/{}",
                        session_id, media.frame_id, media.chunk_index, media.chunk_count,
                    );

                    continue;
                }

                println!(
                    "event=fake_frame_received session={} frame={} chunk={}/{} payload_bytes={} peer={}",
                    session_id,
                    media.frame_id,
                    media.chunk_index,
                    media.chunk_count,
                    media.payload.len(),
                    connection.remote_address(),
                );
            }

            Err(error) => {
                println!(
                    "event=media_receive_finished session={} reason={}",
                    session_id, error,
                );

                break;
            }
        }
    }

    println!(
        "event=client_disconnected session={} connection={}",
        session_id,
        connection.stable_id(),
    );

    Ok(())
}
