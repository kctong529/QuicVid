use crate::media::{fragment_frame, MEDIA_HEADER_SIZE};
use crate::{control, test_pattern, tls};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

pub async fn run(
    connect: SocketAddr,
    bind: SocketAddr,
    fps: u32,
    duration_seconds: u64,
    jpeg_quality: u8,
) -> anyhow::Result<()> {
    if fps == 0 {
        anyhow::bail!("fps must be greater than zero");
    }

    if duration_seconds == 0 {
        anyhow::bail!("duration-seconds must be greater than zero");
    }

    if !(1..=100).contains(&jpeg_quality) {
        anyhow::bail!("JPEG quality must be between 1 and 100");
    }

    let total_frames = u64::from(fps)
        .checked_mul(duration_seconds)
        .ok_or_else(|| anyhow::anyhow!("fake-video frame count overflow"))?;

    println!(
        "event=jpeg_video_config fps={} duration_seconds={} jpeg_quality={} expected_frames={}",
        fps, duration_seconds, jpeg_quality, total_frames,
    );

    let mut endpoint = quinn::Endpoint::client(bind)?;
    endpoint.set_default_client_config(tls::client_config()?);

    println!(
        "event=client_endpoint_created bind={} local={}",
        bind,
        endpoint.local_addr()?
    );

    let session_id = Uuid::new_v4();

    println!("event=session_created session={session_id}");
    println!("event=connecting remote={connect}");

    let connection = endpoint.connect(connect, "localhost")?.await?;

    println!(
        "event=connected session={} connection={} local={} remote={}",
        session_id,
        connection.stable_id(),
        endpoint.local_addr()?,
        connection.remote_address(),
    );

    // Initial QuicVid control handshake.
    let (mut send, mut recv) = connection.open_bi().await?;
    let hello = control::hello(session_id);

    send.write_all(hello.as_bytes()).await?;
    send.finish()?;

    println!("event=hello_sent session={session_id}");

    let response = recv.read_to_end(1024).await?;
    let response = String::from_utf8(response)?;

    control::validate_acknowledgement(&response, session_id)?;

    println!("event=hello_acknowledged session={session_id}");

    let max_datagram_size = connection
        .max_datagram_size()
        .ok_or_else(|| anyhow::anyhow!("QUIC DATAGRAM support is unavailable"))?;

    let max_payload_size = max_datagram_size
        .checked_sub(MEDIA_HEADER_SIZE)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "QUIC DATAGRAM maximum {} is smaller than the {}-byte media header",
                max_datagram_size,
                MEDIA_HEADER_SIZE
            )
        })?;

    println!(
        "event=datagram_transport_ready session={} max_datagram_size={} max_payload_size={}",
        session_id, max_datagram_size, max_payload_size,
    );

    let frame_interval = Duration::from_secs_f64(1.0 / f64::from(fps));
    let mut ticker = tokio::time::interval(frame_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut sent_frames = 0u64;
    let mut sent_datagrams = 0u64;

    for frame_id in 0..total_frames {
        ticker.tick().await;

        let jpeg = test_pattern::generate_jpeg_frame(frame_id, jpeg_quality)?;

        let sent_at_ms = unix_time_ms()?;

        let chunks = fragment_frame(session_id, frame_id, sent_at_ms, &jpeg, max_payload_size)?;

        println!(
            "event=jpeg_frame_encoded session={} frame={} jpeg_bytes={} chunks={}",
            session_id,
            frame_id,
            jpeg.len(),
            chunks.len(),
        );

        for media in chunks {
            let encoded = media.encode()?;

            connection.send_datagram(encoded.into())?;

            sent_datagrams += 1;

            println!(
                "event=media_chunk_sent session={} frame={} chunk={}/{} payload_bytes={}",
                session_id,
                media.frame_id,
                media.chunk_index,
                media.chunk_count,
                media.payload.len(),
            );
        }

        sent_frames += 1;
    }

    println!(
        "event=jpeg_video_send_summary session={} frames={} datagrams={} fps={} duration_seconds={} jpeg_quality={}",
        session_id,
        sent_frames,
        sent_datagrams,
        fps,
        duration_seconds,
        jpeg_quality,
    );

    // Send the authoritative final frame count on a second control stream.
    let (mut done_send, mut done_recv) = connection.open_bi().await?;
    let done = control::done(session_id, sent_frames);

    done_send.write_all(done.as_bytes()).await?;
    done_send.finish()?;

    println!(
        "event=jpeg_video_done_sent session={} frames={}",
        session_id, sent_frames,
    );

    let response = done_recv.read_to_end(1024).await?;
    let response = String::from_utf8(response)?;

    control::validate_done_acknowledgement(&response, session_id)?;

    println!(
        "event=jpeg_video_done_acknowledged session={} frames={}",
        session_id, sent_frames,
    );
    connection.close(0u32.into(), b"JPEG video complete");
    endpoint.wait_idle().await;

    println!("event=client_stopped session={session_id}");

    Ok(())
}

fn unix_time_ms() -> anyhow::Result<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(elapsed.as_millis().try_into()?)
}
