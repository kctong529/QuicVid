use crate::media::{MediaDatagram, MEDIA_HEADER_SIZE};
use crate::{control, tls};
use std::net::SocketAddr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

pub async fn run(
    connect: SocketAddr,
    bind: SocketAddr,
    fps: u32,
    duration_seconds: u64,
    payload_size: usize,
) -> anyhow::Result<()> {
    if fps == 0 {
        anyhow::bail!("fps must be greater than zero");
    }

    if duration_seconds == 0 {
        anyhow::bail!("duration-seconds must be greater than zero");
    }

    let total_frames = u64::from(fps)
        .checked_mul(duration_seconds)
        .ok_or_else(|| anyhow::anyhow!("fake-video frame count overflow"))?;

    println!(
        "event=fake_video_config fps={} duration_seconds={} payload_size={} expected_frames={}",
        fps, duration_seconds, payload_size, total_frames,
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

    if payload_size > max_payload_size {
        anyhow::bail!(
            "payload size {} exceeds current single-datagram media payload limit {}",
            payload_size,
            max_payload_size
        );
    }

    println!(
        "event=datagram_transport_ready session={} max_datagram_size={} max_payload_size={}",
        session_id, max_datagram_size, max_payload_size,
    );

    let frame_interval = Duration::from_secs_f64(1.0 / f64::from(fps));

    let mut ticker = tokio::time::interval(frame_interval);

    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut sent = 0u64;

    for frame_id in 0..total_frames {
        ticker.tick().await;

        let media = MediaDatagram {
            session_id,
            frame_id,
            sent_at_ms: unix_time_ms()?,
            chunk_index: 0,
            chunk_count: 1,
            payload: fake_payload(frame_id, payload_size),
        };

        let encoded = media.encode()?;

        connection.send_datagram(encoded.into())?;

        sent += 1;

        println!(
            "event=fake_frame_sent session={} frame={} chunk={}/{} payload_bytes={}",
            session_id,
            media.frame_id,
            media.chunk_index,
            media.chunk_count,
            media.payload.len(),
        );
    }

    println!(
        "event=fake_video_send_complete session={} sent={}",
        session_id, sent,
    );

    connection.close(0u32.into(), b"fake video complete");

    endpoint.wait_idle().await;

    println!("event=client_stopped session={session_id}");

    Ok(())
}

fn unix_time_ms() -> anyhow::Result<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;

    Ok(elapsed.as_millis().try_into()?)
}

fn fake_payload(frame_id: u64, payload_size: usize) -> Vec<u8> {
    vec![(frame_id & 0xff) as u8; payload_size]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_payload_uses_low_byte_of_frame_id() {
        assert_eq!(fake_payload(258, 4), vec![2, 2, 2, 2]);
    }

    #[test]
    fn fake_payload_has_requested_size() {
        assert_eq!(fake_payload(42, 256).len(), 256);
    }
}
