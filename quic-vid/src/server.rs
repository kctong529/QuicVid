use crate::{control, frame_tracker::FrameTracker, media::MediaDatagram, tls};
use quinn::Connection;
use std::{net::SocketAddr, time::Duration};
use uuid::Uuid;

const POST_DONE_DRAIN: Duration = Duration::from_millis(200);

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
                                        eprintln!("event=connection_error error={error:#}");
                                    }
                                }
                                Err(error) => {
                                    eprintln!("event=handshake_failed error={error}");
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

    let session_started = std::time::Instant::now();
    let mut tracker = FrameTracker::default();

    // Receive media until the client opens the second control stream with DONE.
    let (expected_frames, mut done_send) = loop {
        tokio::select! {
            datagram = connection.read_datagram() => {
                match datagram {
                    Ok(bytes) => {
                        handle_media_datagram(
                            &bytes,
                            session_id,
                            &connection,
                            &mut tracker,
                            session_started.elapsed(),
                        );
                    }
                    Err(error) => {
                        anyhow::bail!("media receive failed before DONE: {error}");
                    }
                }
            }

            control_stream = connection.accept_bi() => {
                let (send, mut recv) = control_stream?;
                let request = recv.read_to_end(1024).await?;
                let request = String::from_utf8(request)?;
                let (done_session_id, expected_frames) = control::parse_done(&request)?;

                if done_session_id != session_id {
                    anyhow::bail!(
                        "DONE session mismatch: expected {}, got {}",
                        session_id,
                        done_session_id
                    );
                }

                println!(
                    "event=fake_video_done session={} expected_frames={}",
                    session_id,
                    expected_frames,
                );

                break (expected_frames, send);
            }
        }
    };

    // Streams and DATAGRAMs do not provide cross-type ordering. Give any final
    // in-flight media a short chance to arrive before declaring it missing.
    if !tracker.has_received_all_expected(expected_frames) {
        let drain_deadline = tokio::time::Instant::now() + POST_DONE_DRAIN;

        loop {
            if tracker.has_received_all_expected(expected_frames) {
                break;
            }

            tokio::select! {
                datagram = connection.read_datagram() => {
                    match datagram {
                        Ok(bytes) => {
                            handle_media_datagram(
                                &bytes,
                                session_id,
                                &connection,
                                &mut tracker,
                                session_started.elapsed(),
                            );
                        }
                        Err(_) => break,
                    }
                }

                _ = tokio::time::sleep_until(drain_deadline) => {
                    break;
                }
            }
        }
    }

    println!("event=fake_video_drain_complete session={session_id}");

    let summary = tracker.summary();
    let missing = tracker.missing_from_expected(expected_frames);

    println!(
        "event=fake_video_receive_summary session={} expected={} received={} unique={} missing={} out_of_order={} duplicates={} largest_gap_ms={}",
        session_id,
        expected_frames,
        summary.received,
        summary.unique,
        missing,
        summary.out_of_order,
        summary.duplicates,
        summary.largest_receive_gap.as_millis(),
    );

    let response = control::done_acknowledgement(session_id);
    done_send.write_all(response.as_bytes()).await?;
    done_send.finish()?;

    println!("event=fake_video_done_acknowledged session={session_id}");

    let close_reason = connection.closed().await;

    println!(
        "event=client_disconnected session={} connection={} reason={}",
        session_id,
        connection.stable_id(),
        close_reason,
    );

    Ok(())
}

fn handle_media_datagram(
    bytes: &[u8],
    session_id: Uuid,
    connection: &Connection,
    tracker: &mut FrameTracker,
    received_at: Duration,
) {
    let media = match MediaDatagram::decode(bytes) {
        Ok(media) => media,
        Err(error) => {
            eprintln!(
                "event=media_datagram_invalid connection={} error={error:#}",
                connection.stable_id(),
            );
            return;
        }
    };

    if media.session_id != session_id {
        eprintln!(
            "event=media_session_mismatch expected={} got={} frame={}",
            session_id, media.session_id, media.frame_id,
        );
        return;
    }

    if media.chunk_index != 0 || media.chunk_count != 1 {
        eprintln!(
            "event=fake_frame_chunk_invalid session={} frame={} chunk={}/{}",
            session_id, media.frame_id, media.chunk_index, media.chunk_count,
        );
        return;
    }

    tracker.record(media.frame_id, received_at);

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
