use crate::media::{fragment_frame, MEDIA_HEADER_SIZE};
use crate::migration::{MigrationContext, MigrationController, MigrationReason, MigrationState};
use crate::path_health::{PathHealthEvent, PathHealthMonitor};
use crate::{control, test_pattern, tls};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const PATH_HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(25);

pub async fn run(
    connect: SocketAddr,
    bind: SocketAddr,
    rebind: Option<SocketAddr>,
    rebind_after_seconds: Option<f64>,
    fps: u32,
    duration_seconds: u64,
    jpeg_quality: u8,
    auto_migrate: bool,
    suspect_after_ms: u64,
    challenge_after_ms: u64,
) -> anyhow::Result<()> {
    if fps == 0 {
        anyhow::bail!("fps must be greater than zero");
    }

    if duration_seconds == 0 {
        anyhow::bail!("duration-seconds must be greater than zero");
    }

    validate_migration_config(
        rebind,
        rebind_after_seconds,
        auto_migrate,
        suspect_after_ms,
        challenge_after_ms,
    )?;

    if let Some(rebind_after) = rebind_after_seconds {
        if rebind_after <= 0.0 {
            anyhow::bail!("rebind-after-seconds must be greater than zero");
        }

        if rebind_after >= duration_seconds as f64 {
            anyhow::bail!("rebind-after-seconds must be smaller than duration-seconds");
        }
    }

    if auto_migrate {
        println!(
            "event=path_health_config \
             enabled=true \
             suspect_after_ms={} \
             challenge_after_ms={}",
            suspect_after_ms, challenge_after_ms
        );
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

    let frame_interval = 1.0 / f64::from(fps);
    let video_started = tokio::time::Instant::now();
    let health_started = Instant::now();

    let initial_ack_count = connection.stats().frame_rx.acks;

    let mut path_health = auto_migrate.then(|| {
        PathHealthMonitor::new(
            Duration::from_millis(suspect_after_ms),
            Duration::from_millis(challenge_after_ms),
            health_started,
            initial_ack_count,
        )
    });

    let mut next_health_check = health_started;

    let mut sent_frames = 0u64;
    let mut sent_datagrams = 0u64;
    let mut last_frame_id = None;
    let mut rebound = false;
    let mut migration = MigrationController::new();

    println!(
        "event=migration_controller_ready \
         state={} \
         connection={} \
         local={}",
        migration.state(),
        connection.stable_id(),
        endpoint.local_addr()?,
    );

    loop {
        let elapsed = video_started.elapsed().as_secs_f64();

        if let Some(monitor) = path_health.as_mut() {
            let now = Instant::now();

            if now >= next_health_check {
                let ack_count = connection.stats().frame_rx.acks;

                match monitor.observe(now, ack_count) {
                    PathHealthEvent::None => {}

                    PathHealthEvent::BecameSuspect => {
                        println!(
                            "event=path_health \
                             status=suspect \
                             reason=ack_progress_timeout \
                             elapsed_seconds={:.3} \
                             ack_count={} \
                             suspect_after_ms={}",
                            elapsed, ack_count, suspect_after_ms,
                        );
                    }

                    PathHealthEvent::Recovered => {
                        println!(
                            "event=path_health \
                             status=recovered \
                             reason=ack_progress_resumed \
                             elapsed_seconds={:.3} \
                             ack_count={}",
                            elapsed, ack_count,
                        );
                    }

                    PathHealthEvent::ChallengeRequested => {
                        println!(
                            "event=path_health \
                             status=challenge_requested \
                             reason=ack_progress_timeout_persisted \
                             elapsed_seconds={:.3} \
                             ack_count={} \
                             challenge_after_ms={}",
                            elapsed, ack_count, challenge_after_ms,
                        );
                    }
                }

                next_health_check = now + PATH_HEALTH_POLL_INTERVAL;
            }
        }

        if !rebound {
            if let (Some(rebind_addr), Some(rebind_after)) = (rebind, rebind_after_seconds) {
                if elapsed >= rebind_after {
                    let old_addr = endpoint.local_addr()?;
                    let elapsed_duration = video_started.elapsed();

                    println!(
                        "event=migration_requested \
                         elapsed_seconds={:.3} \
                         old_local={} \
                         requested_local={}",
                        elapsed, old_addr, rebind_addr
                    );

                    let context = MigrationContext {
                        elapsed: elapsed_duration,
                        active_local: old_addr,
                        candidate_local: Some(rebind_addr),
                        connection_id: connection.stable_id(),
                    };

                    migration.transition(
                        MigrationState::Suspect,
                        MigrationReason::ControlledTrigger,
                        context,
                    )?;

                    migration.transition(
                        MigrationState::Challenging,
                        MigrationReason::ConditionPersisted,
                        context,
                    )?;

                    migration.transition(
                        MigrationState::Migrating,
                        MigrationReason::AlternatePathReady,
                        context,
                    )?;

                    let new_addr = rebind_endpoint(&endpoint, rebind_addr)?;

                    println!(
                        "event=endpoint_rebound \
                         elapsed_seconds={:.3} \
                         old_local={} \
                         new_local={} \
                         connection={}",
                        elapsed,
                        old_addr,
                        new_addr,
                        connection.stable_id()
                    );

                    migration.transition(
                        MigrationState::Healthy,
                        MigrationReason::MigrationCompleted,
                        MigrationContext {
                            elapsed: video_started.elapsed(),
                            active_local: new_addr,
                            candidate_local: None,
                            connection_id: connection.stable_id(),
                        },
                    )?;

                    rebound = true;
                }
            }
        }

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

fn validate_migration_config(
    rebind: Option<SocketAddr>,
    rebind_after_seconds: Option<f64>,
    auto_migrate: bool,
    suspect_after_ms: u64,
    challenge_after_ms: u64,
) -> anyhow::Result<()> {
    match (rebind, rebind_after_seconds) {
        (Some(_), None) => {
            anyhow::bail!("--rebind requires --rebind-after-seconds");
        }
        (None, Some(_)) => {
            anyhow::bail!("--rebind-after-seconds requires --rebind");
        }
        _ => {}
    }

    if auto_migrate {
        if suspect_after_ms == 0 {
            anyhow::bail!("--suspect-after-ms must be greater than zero");
        }

        if challenge_after_ms == 0 {
            anyhow::bail!("--challenge-after-ms must be greater than zero");
        }

        if rebind.is_some() || rebind_after_seconds.is_some() {
            anyhow::bail!("--auto-migrate cannot be combined with timed migration");
        }
    }

    Ok(())
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

    #[test]
    fn rebind_requires_rebind_time() {
        let result = validate_migration_config(
            Some("127.0.0.1:5000".parse().unwrap()),
            None,
            false,
            250,
            250,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rebind_time_requires_rebind_address() {
        let result = validate_migration_config(None, Some(1.0), false, 250, 250);

        assert!(result.is_err());
    }

    #[test]
    fn automatic_migration_rejects_zero_suspect_threshold() {
        let result = validate_migration_config(None, None, true, 0, 250);

        assert!(result.is_err());
    }

    #[test]
    fn automatic_migration_rejects_zero_challenge_threshold() {
        let result = validate_migration_config(None, None, true, 250, 0);

        assert!(result.is_err());
    }

    #[test]
    fn automatic_and_timed_migration_are_mutually_exclusive() {
        let result = validate_migration_config(
            Some("127.0.0.1:5000".parse().unwrap()),
            Some(1.0),
            true,
            250,
            250,
        );

        assert!(result.is_err());
    }

    #[test]
    fn automatic_migration_without_fallback_address_is_valid() {
        let result = validate_migration_config(None, None, true, 250, 250);

        assert!(result.is_ok());
    }

    #[test]
    fn controlled_timed_migration_remains_valid() {
        let result = validate_migration_config(
            Some("127.0.0.1:5000".parse().unwrap()),
            Some(1.0),
            false,
            250,
            250,
        );

        assert!(result.is_ok());
    }
}
