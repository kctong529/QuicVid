use crate::preview::{PreviewJpeg, PreviewSender};
use crate::test_pattern::{TEST_FRAME_HEIGHT, TEST_FRAME_WIDTH};
use crate::{
    control, frame_assembler::FrameAssembler, frame_tracker::FrameTracker, media::MediaDatagram,
    tls,
};
use image::GenericImageView;
use quinn::Connection;
use std::{net::SocketAddr, time::Duration};
use uuid::Uuid;

const POST_DONE_DRAIN: Duration = Duration::from_millis(200);
const FRAME_ASSEMBLY_TIMEOUT: Duration = Duration::from_secs(1);

pub async fn run(listen: SocketAddr, preview_sender: Option<PreviewSender>) -> anyhow::Result<()> {
    println!("server_preview enabled={}", preview_sender.is_some());

    let server_config = tls::server_config()?;
    let endpoint = quinn::Endpoint::server(server_config, listen)?;

    println!("event=server_started listen={}", endpoint.local_addr()?);

    loop {
        tokio::select! {
            incoming = endpoint.accept() => {
                match incoming {
                    Some(incoming) => {
                        let connection_preview_sender = preview_sender.clone();

                        tokio::spawn(async move {
                            match incoming.await {
                                Ok(connection) => {
                                    if let Err(error) =
                                        handle_connection(
                                            connection,
                                            connection_preview_sender,
                                        ).await
                                    {
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

async fn handle_connection(
    connection: Connection,
    preview_sender: Option<PreviewSender>,
) -> anyhow::Result<()> {
    println!(
        "event=client_connected connection={} peer={}",
        connection.stable_id(),
        connection.remote_address(),
    );

    let (mut send, mut recv) = connection.accept_bi().await?;
    let request = recv.read_to_end(1024).await?;
    let request = String::from_utf8(request)?;
    let hello = control::parse_hello(&request)?;
    let media_run_id = hello.media_run_id;
    let session_id = hello.session_id;

    println!(
        "event=client_hello media_run={} session={} connection={} peer={}",
        media_run_id,
        session_id,
        connection.stable_id(),
        connection.remote_address(),
    );

    let response = control::acknowledgement(media_run_id, session_id);
    send.write_all(response.as_bytes()).await?;
    send.finish()?;

    println!(
        "event=hello_acknowledged media_run={} session={} connection={}",
        media_run_id,
        session_id,
        connection.stable_id(),
    );

    let session_started = std::time::Instant::now();
    let mut tracker = FrameTracker::default();
    let mut assembler = FrameAssembler::default();

    let (expected_frames, mut done_send) = loop {
        tokio::select! {
            datagram = connection.read_datagram() => {
                match datagram {
                    Ok(bytes) => {
                        handle_media_datagram(
                            &bytes,
                            session_id,
                            &connection,
                            &mut assembler,
                            &mut tracker,
                            session_started.elapsed(),
                            preview_sender.as_ref(),
                        );
                    }

                    Err(error) => {
                        anyhow::bail!(
                            "media receive failed before DONE: {error}"
                        );
                    }
                }
            }

            control_stream = connection.accept_bi() => {
                let (send, mut recv) = control_stream?;
                let request = recv.read_to_end(1024).await?;
                let request = String::from_utf8(request)?;
                let (done_session_id, expected_frames) =
                    control::parse_done(&request)?;

                if done_session_id != session_id {
                    anyhow::bail!(
                        "DONE session mismatch: expected {}, got {}",
                        session_id,
                        done_session_id
                    );
                }

                println!(
                    "event=jpeg_video_done session={} expected_frames={}",
                    session_id,
                    expected_frames,
                );

                break (expected_frames, send);
            }
        }
    };

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
                                &mut assembler,
                                &mut tracker,
                                session_started.elapsed(),
                                preview_sender.as_ref(),
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

    println!("event=jpeg_video_drain_complete session={session_id}");

    let summary = tracker.summary();
    let missing = tracker.missing_from_expected(expected_frames);

    println!(
        "event=jpeg_video_receive_summary session={} expected={} received={} unique={} missing={} out_of_order={} duplicates={} largest_gap_ms={} incomplete_frames={}",
        session_id,
        expected_frames,
        summary.received,
        summary.unique,
        missing,
        summary.out_of_order,
        summary.duplicates,
        summary.largest_receive_gap.as_millis(),
        assembler.incomplete_frame_count(),
    );

    let response = control::done_acknowledgement(session_id);
    done_send.write_all(response.as_bytes()).await?;
    done_send.finish()?;

    println!("event=jpeg_video_done_acknowledged session={session_id}");

    let close_reason = connection.closed().await;

    println!(
        "event=client_disconnected session={} connection={} reason={}",
        session_id,
        connection.stable_id(),
        close_reason,
    );

    Ok(())
}

fn validate_jpeg_frame(bytes: &[u8]) -> anyhow::Result<(u32, u32)> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg)?;

    let dimensions = image.dimensions();

    if dimensions != (TEST_FRAME_WIDTH, TEST_FRAME_HEIGHT) {
        anyhow::bail!(
            "unexpected JPEG dimensions: expected {}x{}, got {}x{}",
            TEST_FRAME_WIDTH,
            TEST_FRAME_HEIGHT,
            dimensions.0,
            dimensions.1,
        );
    }

    Ok(dimensions)
}

fn handle_media_datagram(
    bytes: &[u8],
    session_id: Uuid,
    connection: &Connection,
    assembler: &mut FrameAssembler,
    tracker: &mut FrameTracker,
    received_at: Duration,
    preview_sender: Option<&PreviewSender>,
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

    println!(
        "event=media_chunk_received session={} frame={} chunk={}/{} payload_bytes={} peer={}",
        session_id,
        media.frame_id,
        media.chunk_index,
        media.chunk_count,
        media.payload.len(),
        connection.remote_address(),
    );

    match assembler.push(media, received_at) {
        Ok(Some(frame)) => {
            let (width, height) = match validate_jpeg_frame(&frame.bytes) {
                Ok(dimensions) => dimensions,

                Err(error) => {
                    eprintln!(
                            "event=jpeg_frame_invalid session={} frame={} jpeg_bytes={} error={error:#}",
                            frame.session_id,
                            frame.frame_id,
                            frame.bytes.len(),
                        );

                    return;
                }
            };

            tracker.record(frame.frame_id, received_at);

            if let Some(sender) = preview_sender {
                let preview_jpeg = PreviewJpeg {
                    frame_id: frame.frame_id,
                    bytes: frame.bytes.clone(),
                };

                if let Err(error) = crate::preview::publish(sender, preview_jpeg) {
                    eprintln!(
                        "event=preview_publish_failed frame={} error={error}",
                        frame.frame_id,
                    );
                }
            }

            println!(
                "event=jpeg_frame_reassembled session={} frame={} jpeg_bytes={} sent_at_ms={} peer={}",
                frame.session_id,
                frame.frame_id,
                frame.bytes.len(),
                frame.sent_at_ms,
                connection.remote_address(),
            );

            println!(
                "event=jpeg_frame_validated session={} frame={} width={} height={} jpeg_bytes={}",
                frame.session_id,
                frame.frame_id,
                width,
                height,
                frame.bytes.len(),
            );
        }

        Ok(None) => {}

        Err(error) => {
            eprintln!(
                "event=frame_reassembly_error session={} connection={} error={error:#}",
                session_id,
                connection.stable_id(),
            );
        }
    }

    let expired = assembler.discard_stale(received_at, FRAME_ASSEMBLY_TIMEOUT);

    if expired > 0 {
        println!(
            "event=incomplete_frames_expired session={} count={} remaining={}",
            session_id,
            expired,
            assembler.incomplete_frame_count(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_generated_jpeg_frame() {
        let jpeg =
            crate::test_pattern::generate_jpeg_frame(42, crate::test_pattern::DEFAULT_JPEG_QUALITY)
                .unwrap();

        let dimensions = validate_jpeg_frame(&jpeg).unwrap();

        assert_eq!(dimensions, (TEST_FRAME_WIDTH, TEST_FRAME_HEIGHT));
    }

    #[test]
    fn rejects_non_jpeg_bytes() {
        assert!(validate_jpeg_frame(b"not a jpeg").is_err());
    }
}
