use crate::media::{fragment_frame, MEDIA_HEADER_SIZE};
use crate::{control, test_pattern, tls};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub async fn run(
    connect: SocketAddr,
    bind: SocketAddr,
    rebind: Option<SocketAddr>,
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

    if let Some(rebind_addr) = rebind {
        let old_addr = endpoint.local_addr()?;

        println!(
            "event=migration_requested old_local={} requested_local={}",
            old_addr, rebind_addr
        );

        let new_addr = rebind_endpoint(&endpoint, rebind_addr)?;

        println!(
            "event=endpoint_rebound old_local={} new_local={}",
            old_addr, new_addr
        );
    }

    let frame_interval = 1.0 / f64::from(fps);
    let video_started = tokio::time::Instant::now();

    let mut sent_frames = 0u64;
    let mut sent_datagrams = 0u64;
    let mut last_frame_id = None;

    loop {
        let elapsed = video_started.elapsed().as_secs_f64();
        let frame_id = (elapsed / frame_interval).floor() as u64;

        if frame_id >= total_frames {
            break;
        }

        if last_frame_id == Some(frame_id) {
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
        }

        last_frame_id = Some(frame_id);

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

fn rebind_endpoint(endpoint: &quinn::Endpoint, bind: SocketAddr) -> anyhow::Result<SocketAddr> {
    let socket = UdpSocket::bind(bind)?;
    let local_addr = socket.local_addr()?;

    endpoint.rebind(socket)?;

    Ok(local_addr)
}

fn unix_time_ms() -> anyhow::Result<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
    Ok(elapsed.as_millis().try_into()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rebind_endpoint_updates_endpoint_local_address() {
        let endpoint = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();

        let old_addr = endpoint.local_addr().unwrap();

        let new_addr = rebind_endpoint(&endpoint, "127.0.0.1:0".parse().unwrap()).unwrap();

        assert_eq!(new_addr.ip(), old_addr.ip());
        assert_ne!(new_addr.port(), old_addr.port());
        assert_eq!(endpoint.local_addr().unwrap(), new_addr);
    }
}
