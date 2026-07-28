use crate::media::{fragment_frame, MEDIA_HEADER_SIZE};
use crate::media_run::MediaRun;
use crate::migration::{MigrationContext, MigrationController, MigrationReason, MigrationState};
use crate::path_discovery;
use crate::path_health::{PathHealthEvent, PathHealthMonitor};
use crate::{control, test_pattern, tls};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const PATH_HEALTH_POLL_INTERVAL: Duration = Duration::from_millis(25);

struct TransportSession {
    endpoint: quinn::Endpoint,
    connection: quinn::Connection,
    session_id: Uuid,
    max_payload_size: usize,
}

impl TransportSession {
    fn local_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.endpoint.local_addr()?)
    }

    fn connection_id(&self) -> usize {
        self.connection.stable_id()
    }
}

async fn connect_session(
    connect: SocketAddr,
    bind: SocketAddr,
    media_run_id: Uuid,
) -> anyhow::Result<TransportSession> {
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
    let hello = control::hello(media_run_id, session_id);

    send.write_all(hello.as_bytes()).await?;
    send.finish()?;

    println!(
        "event=hello_sent media_run={} session={session_id}",
        media_run_id
    );

    let response = recv.read_to_end(1024).await?;
    let response = String::from_utf8(response)?;

    control::validate_acknowledgement(&response, media_run_id, session_id)?;

    println!(
        "event=hello_acknowledged media_run={} session={session_id}",
        media_run_id
    );

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

    Ok(TransportSession {
        endpoint,
        connection,
        session_id,
        max_payload_size,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum RecoveryStrategy {
    Migrate,
    Reconnect,
}

impl RecoveryStrategy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Migrate => "migrate",
            Self::Reconnect => "reconnect",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AlternativeDiscoveryResult {
    Selected(path_discovery::PathCandidate),
    NoAlternative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathHealthAction {
    None,
    RecoverUsing(RecoveryStrategy),
}

fn discover_alternative_path(
    active_local: SocketAddr,
) -> anyhow::Result<AlternativeDiscoveryResult> {
    let active_ip = match active_local.ip() {
        std::net::IpAddr::V4(ip) => ip,
        std::net::IpAddr::V6(_) => {
            anyhow::bail!("automatic path discovery currently supports IPv4 only");
        }
    };

    println!(
        "event=path_discovery_started \
         active_local={}",
        active_local
    );

    let candidates = path_discovery::discover_ipv4_candidates(active_ip)?;

    for candidate in &candidates {
        println!(
            "event=path_candidate_found \
             interface={} \
             candidate_ip={}",
            candidate.interface_name, candidate.local_ip,
        );
    }

    match path_discovery::select_candidate(&candidates) {
        Some(candidate) => {
            println!(
                "event=path_candidate_selected \
                 interface={} \
                 candidate_ip={}",
                candidate.interface_name, candidate.local_ip,
            );

            Ok(AlternativeDiscoveryResult::Selected(candidate.clone()))
        }

        None => {
            println!(
                "event=path_discovery_failed \
                 reason=no_alternative \
                 active_local={}",
                active_local
            );

            Ok(AlternativeDiscoveryResult::NoAlternative)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    connect: SocketAddr,
    bind: SocketAddr,
    rebind: Option<SocketAddr>,
    rebind_after_seconds: Option<f64>,
    fps: u32,
    duration_seconds: u64,
    jpeg_quality: u8,
    auto_migrate: bool,
    recovery_strategy: RecoveryStrategy,
    suspect_after_ms: u64,
    challenge_after_ms: u64,
    quiet_media_logs: bool,
    quiet_datagram_logs: bool,
) -> anyhow::Result<()> {
    let log_frames = !quiet_media_logs;
    let log_datagrams = !quiet_media_logs && !quiet_datagram_logs;

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
        recovery_strategy,
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

        println!(
            "event=recovery_strategy_config strategy={}",
            recovery_strategy.as_str()
        );
    }

    if !(1..=100).contains(&jpeg_quality) {
        anyhow::bail!("JPEG quality must be between 1 and 100");
    }

    let media_run = MediaRun::new(fps, Duration::from_secs(duration_seconds))?;
    let total_frames = media_run.total_frames();

    println!(
        "event=media_run_created media_run={} fps={} duration_seconds={} expected_frames={}",
        media_run.id(),
        media_run.fps(),
        media_run.duration().as_secs(),
        total_frames,
    );

    println!(
        "event=jpeg_video_config media_run={} fps={} duration_seconds={} jpeg_quality={} expected_frames={}",
        media_run.id(), fps, duration_seconds, jpeg_quality, total_frames,
    );

    let mut transport = connect_session(connect, bind, media_run.id()).await?;

    let health_started = Instant::now();

    let initial_ack_count = transport.connection.stats().frame_rx.acks;

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
    let mut automatic_migration_pending = false;
    let mut migration = MigrationController::new();

    println!(
        "event=migration_controller_ready \
         state={} \
         connection={} \
         local={}",
        migration.state(),
        transport.connection_id(),
        transport.local_addr()?,
    );

    loop {
        let now = Instant::now();
        let media_elapsed = media_run.elapsed(now);
        let elapsed = media_elapsed.as_secs_f64();

        if let Some(monitor) = path_health.as_mut() {
            let now = Instant::now();

            if now >= next_health_check {
                let ack_count = transport.connection.stats().frame_rx.acks;

                let event = monitor.observe(now, ack_count);

                match event {
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

                if event != PathHealthEvent::None {
                    let active_local = transport.local_addr()?;
                    let state_before_event = migration.state();

                    let action = handle_path_health_event(
                        event,
                        recovery_strategy,
                        &mut migration,
                        media_elapsed,
                        active_local,
                        transport.connection_id(),
                    )?;

                    if event == PathHealthEvent::Recovered
                        && state_before_event == MigrationState::Migrating
                    {
                        println!(
                            "event=migration_confirmed \
                             elapsed_seconds={:.3} \
                             active_local={} \
                             ack_count={} \
                             connection={}",
                            media_run.elapsed(Instant::now()).as_secs_f64(),
                            transport.local_addr()?,
                            ack_count,
                            transport.connection_id(),
                        );

                        automatic_migration_pending = false;
                    }

                    if !automatic_migration_pending {
                        if let PathHealthAction::RecoverUsing(strategy) = action {
                            match discover_alternative_path(active_local)? {
                                AlternativeDiscoveryResult::Selected(candidate) => match strategy {
                                    RecoveryStrategy::Migrate => {
                                        let old_local = transport.local_addr()?;
                                        let requested_local =
                                            SocketAddr::new(candidate.local_ip.into(), 0);

                                        println!(
                                            "event=automatic_migration_candidate_ready \
                                         interface={} \
                                         candidate_local={} \
                                         connection={}",
                                            candidate.interface_name,
                                            requested_local,
                                            transport.connection_id(),
                                        );

                                        println!(
                                            "event=automatic_rebind_started \
                                         elapsed_seconds={:.3} \
                                         old_local={} \
                                         requested_local={} \
                                         interface={} \
                                         connection={}",
                                            media_run.elapsed(Instant::now()).as_secs_f64(),
                                            old_local,
                                            requested_local,
                                            candidate.interface_name,
                                            transport.connection_id(),
                                        );

                                        let new_local = match rebind_endpoint(
                                            &transport.endpoint,
                                            requested_local,
                                        ) {
                                            Ok(new_local) => new_local,

                                            Err(error) => {
                                                println!(
                                                    "event=automatic_rebind_failed \
                                                 elapsed_seconds={:.3} \
                                                 old_local={} \
                                                 requested_local={} \
                                                 interface={} \
                                                 connection={} \
                                                 error={}",
                                                    media_run.elapsed(Instant::now()).as_secs_f64(),
                                                    old_local,
                                                    requested_local,
                                                    candidate.interface_name,
                                                    transport.connection_id(),
                                                    error,
                                                );

                                                return Err(error);
                                            }
                                        };

                                        migration.transition(
                                            MigrationState::Migrating,
                                            MigrationReason::AlternatePathReady,
                                            MigrationContext {
                                                elapsed: media_run.elapsed(Instant::now()),
                                                active_local: old_local,
                                                candidate_local: Some(new_local),
                                                connection_id: transport.connection_id(),
                                            },
                                        )?;

                                        println!(
                                            "event=endpoint_rebound \
                                         mode=automatic \
                                         elapsed_seconds={:.3} \
                                         old_local={} \
                                         new_local={} \
                                         connection={}",
                                            media_run.elapsed(Instant::now()).as_secs_f64(),
                                            old_local,
                                            new_local,
                                            transport.connection_id(),
                                        );

                                        automatic_migration_pending = true;
                                    }

                                    RecoveryStrategy::Reconnect => {
                                        let requested_local =
                                            SocketAddr::new(candidate.local_ip.into(), 0);
                                        let old_session_id = transport.session_id;
                                        let old_connection_id = transport.connection_id();
                                        let old_local = transport.local_addr()?;

                                        println!(
                                            "event=reconnect_requested \
                                         elapsed_seconds={:.3} \
                                         reason=path_degradation_persisted \
                                         active_local={} \
                                         requested_local={} \
                                         interface={} \
                                         old_session={} \
                                         old_connection={}",
                                            media_run.elapsed(Instant::now()).as_secs_f64(),
                                            active_local,
                                            requested_local,
                                            candidate.interface_name,
                                            old_session_id,
                                            old_connection_id,
                                        );

                                        println!(
                                            "event=reconnect_started \
                                         elapsed_seconds={:.3} \
                                         media_run={} \
                                         old_session={} \
                                         old_connection={} \
                                         old_local={} \
                                         requested_local={} \
                                         interface={}",
                                            media_run.elapsed(Instant::now()).as_secs_f64(),
                                            media_run.id(),
                                            old_session_id,
                                            old_connection_id,
                                            old_local,
                                            requested_local,
                                            candidate.interface_name,
                                        );

                                        let replacement = connect_session(
                                            connect,
                                            requested_local,
                                            media_run.id(),
                                        )
                                        .await?;

                                        let new_session_id = replacement.session_id;
                                        let new_connection_id = replacement.connection_id();
                                        let new_local = replacement.local_addr()?;
                                        let new_ack_count =
                                            replacement.connection.stats().frame_rx.acks;
                                        let health_reset_at = Instant::now();

                                        let old_transport =
                                            std::mem::replace(&mut transport, replacement);

                                        old_transport
                                            .connection
                                            .close(0u32.into(), b"replaced by proactive reconnect");

                                        *monitor = PathHealthMonitor::new(
                                            Duration::from_millis(suspect_after_ms),
                                            Duration::from_millis(challenge_after_ms),
                                            health_reset_at,
                                            new_ack_count,
                                        );
                                        migration = MigrationController::new();
                                        automatic_migration_pending = false;

                                        println!(
                                            "event=reconnect_completed \
                                         elapsed_seconds={:.3} \
                                         media_run={} \
                                         old_session={} \
                                         new_session={} \
                                         old_connection={} \
                                         new_connection={} \
                                         old_local={} \
                                         new_local={}",
                                            media_run.elapsed(Instant::now()).as_secs_f64(),
                                            media_run.id(),
                                            old_session_id,
                                            new_session_id,
                                            old_connection_id,
                                            new_connection_id,
                                            old_local,
                                            new_local,
                                        );
                                    }
                                },

                                AlternativeDiscoveryResult::NoAlternative => {
                                    // Remain in Challenging. Retry policy is outside the current prototype scope.
                                }
                            }
                        }
                    }
                }

                next_health_check = now + PATH_HEALTH_POLL_INTERVAL;
            }
        }

        if !rebound {
            if let (Some(rebind_addr), Some(rebind_after)) = (rebind, rebind_after_seconds) {
                if elapsed >= rebind_after {
                    let old_addr = transport.local_addr()?;
                    let elapsed_duration = media_elapsed;

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
                        connection_id: transport.connection_id(),
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

                    let new_addr = rebind_endpoint(&transport.endpoint, rebind_addr)?;

                    println!(
                        "event=endpoint_rebound \
                         elapsed_seconds={:.3} \
                         old_local={} \
                         new_local={} \
                         connection={}",
                        elapsed,
                        old_addr,
                        new_addr,
                        transport.connection_id()
                    );

                    migration.transition(
                        MigrationState::Healthy,
                        MigrationReason::MigrationCompleted,
                        MigrationContext {
                            elapsed: media_run.elapsed(Instant::now()),
                            active_local: new_addr,
                            candidate_local: None,
                            connection_id: transport.connection_id(),
                        },
                    )?;

                    rebound = true;
                }
            }
        }

        let frame_id = media_run.current_frame_id(now);

        if media_run.is_complete(now) || frame_id >= total_frames {
            break;
        }

        if last_frame_id == Some(frame_id) {
            tokio::time::sleep(Duration::from_millis(1)).await;
            continue;
        }

        last_frame_id = Some(frame_id);

        let jpeg = test_pattern::generate_jpeg_frame(frame_id, jpeg_quality)?;

        let sent_at_ms = unix_time_ms()?;

        let chunks = fragment_frame(
            transport.session_id,
            frame_id,
            sent_at_ms,
            &jpeg,
            transport.max_payload_size,
        )?;

        if log_frames {
            println!(
                "event=jpeg_frame_encoded media_run={} session={} frame={} jpeg_bytes={} chunks={}",
                media_run.id(),
                transport.session_id,
                frame_id,
                jpeg.len(),
                chunks.len(),
            );
        }

        for media in chunks {
            let encoded = media.encode()?;

            transport.connection.send_datagram(encoded.into())?;

            sent_datagrams += 1;

            if log_datagrams {
                println!(
                    "event=media_chunk_submitted media_run={} session={} frame={} chunk={}/{} payload_bytes={}",
                    media_run.id(),
                    transport.session_id,
                    media.frame_id,
                    media.chunk_index,
                    media.chunk_count,
                    media.payload.len(),
                );
            }
        }

        sent_frames += 1;
    }

    println!(
        "event=jpeg_video_send_summary media_run={} session={} frames={} datagrams={} fps={} duration_seconds={} jpeg_quality={}",
        media_run.id(),
        transport.session_id,
        sent_frames,
        sent_datagrams,
        fps,
        duration_seconds,
        jpeg_quality,
    );

    // Send the authoritative final frame count on a second control stream.
    let (mut done_send, mut done_recv) = transport.connection.open_bi().await?;
    let final_frame_exclusive = media_run.total_frames();
    let done = control::done(media_run.id(), transport.session_id, final_frame_exclusive);

    done_send.write_all(done.as_bytes()).await?;
    done_send.finish()?;

    println!(
        "event=jpeg_video_done_sent media_run={} session={} final_frame_exclusive={}",
        media_run.id(),
        transport.session_id,
        final_frame_exclusive,
    );

    let response = done_recv.read_to_end(1024).await?;
    let response = String::from_utf8(response)?;

    control::validate_done_acknowledgement(&response, media_run.id(), transport.session_id)?;

    println!(
        "event=jpeg_video_done_acknowledged media_run={} session={} final_frame_exclusive={}",
        media_run.id(),
        transport.session_id,
        final_frame_exclusive,
    );
    transport
        .connection
        .close(0u32.into(), b"JPEG video complete");
    transport.endpoint.wait_idle().await;

    println!(
        "event=media_run_completed media_run={} final_frame_exclusive={} sessions=1",
        media_run.id(),
        total_frames,
    );
    println!(
        "event=client_stopped media_run={} session={}",
        media_run.id(),
        transport.session_id
    );

    Ok(())
}

fn handle_path_health_event(
    event: PathHealthEvent,
    recovery_strategy: RecoveryStrategy,
    migration: &mut MigrationController,
    elapsed: Duration,
    active_local: SocketAddr,
    connection_id: usize,
) -> anyhow::Result<PathHealthAction> {
    let context = MigrationContext {
        elapsed,
        active_local,
        candidate_local: None,
        connection_id,
    };

    match event {
        PathHealthEvent::None => Ok(PathHealthAction::None),

        PathHealthEvent::BecameSuspect => {
            migration.transition(
                MigrationState::Suspect,
                MigrationReason::AckProgressTimeout,
                context,
            )?;

            Ok(PathHealthAction::None)
        }

        PathHealthEvent::Recovered => {
            let reason = match migration.state() {
                MigrationState::Migrating => MigrationReason::MigrationCompleted,
                MigrationState::Suspect | MigrationState::Challenging => {
                    MigrationReason::AckProgressRecovered
                }
                MigrationState::Healthy => return Ok(PathHealthAction::None),
            };

            migration.transition(MigrationState::Healthy, reason, context)?;

            Ok(PathHealthAction::None)
        }

        PathHealthEvent::ChallengeRequested => {
            migration.transition(
                MigrationState::Challenging,
                MigrationReason::PathDegradationPersisted,
                context,
            )?;

            println!(
                "event=path_challenge_requested \
                 elapsed_seconds={:.3} \
                 reason=ack_progress_timeout_persisted \
                 active_local={} \
                 connection={}",
                elapsed.as_secs_f64(),
                active_local,
                connection_id,
            );

            Ok(PathHealthAction::RecoverUsing(recovery_strategy))
        }
    }
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
    recovery_strategy: RecoveryStrategy,
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

    if recovery_strategy == RecoveryStrategy::Reconnect && !auto_migrate {
        anyhow::bail!("--recovery-strategy reconnect requires --auto-migrate");
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

    const TEST_CONNECTION_ID: usize = 42;
    const SUSPECT_AFTER_MS: u64 = 250;
    const CHALLENGE_AFTER_MS: u64 = 250;

    fn test_local_address() -> SocketAddr {
        "10.0.1.2:5000".parse().unwrap()
    }

    fn move_to_suspect(migration: &mut MigrationController) -> PathHealthAction {
        handle_path_health_event(
            PathHealthEvent::BecameSuspect,
            RecoveryStrategy::Migrate,
            migration,
            Duration::from_millis(250),
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap()
    }

    fn move_to_challenging(migration: &mut MigrationController) -> PathHealthAction {
        move_to_suspect(migration);

        handle_path_health_event(
            PathHealthEvent::ChallengeRequested,
            RecoveryStrategy::Migrate,
            migration,
            Duration::from_millis(750),
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap()
    }

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
            RecoveryStrategy::Migrate,
            SUSPECT_AFTER_MS,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_err());
    }

    #[test]
    fn rebind_time_requires_rebind_address() {
        let result = validate_migration_config(
            None,
            Some(1.0),
            false,
            RecoveryStrategy::Migrate,
            SUSPECT_AFTER_MS,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_err());
    }

    #[test]
    fn automatic_migration_rejects_zero_suspect_threshold() {
        let result = validate_migration_config(
            None,
            None,
            true,
            RecoveryStrategy::Migrate,
            0,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_err());
    }

    #[test]
    fn automatic_migration_rejects_zero_challenge_threshold() {
        let result = validate_migration_config(
            None,
            None,
            true,
            RecoveryStrategy::Migrate,
            SUSPECT_AFTER_MS,
            0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn automatic_and_timed_migration_are_mutually_exclusive() {
        let result = validate_migration_config(
            Some("127.0.0.1:5000".parse().unwrap()),
            Some(1.0),
            true,
            RecoveryStrategy::Migrate,
            SUSPECT_AFTER_MS,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_err());
    }

    #[test]
    fn automatic_migration_without_fallback_address_is_valid() {
        let result = validate_migration_config(
            None,
            None,
            true,
            RecoveryStrategy::Migrate,
            SUSPECT_AFTER_MS,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn controlled_timed_migration_remains_valid() {
        let result = validate_migration_config(
            Some("127.0.0.1:5000".parse().unwrap()),
            Some(1.0),
            false,
            RecoveryStrategy::Migrate,
            SUSPECT_AFTER_MS,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn reconnect_strategy_requires_automatic_path_health() {
        let result = validate_migration_config(
            None,
            None,
            false,
            RecoveryStrategy::Reconnect,
            SUSPECT_AFTER_MS,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_err());
    }

    #[test]
    fn reconnect_strategy_with_automatic_path_health_is_valid() {
        let result = validate_migration_config(
            None,
            None,
            true,
            RecoveryStrategy::Reconnect,
            SUSPECT_AFTER_MS,
            CHALLENGE_AFTER_MS,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn no_path_health_event_produces_no_action() {
        let mut migration = MigrationController::new();

        let action = handle_path_health_event(
            PathHealthEvent::None,
            RecoveryStrategy::Migrate,
            &mut migration,
            Duration::ZERO,
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap();

        assert_eq!(action, PathHealthAction::None);
        assert_eq!(migration.state(), MigrationState::Healthy);
    }

    #[test]
    fn suspect_event_moves_controller_to_suspect_without_discovery() {
        let mut migration = MigrationController::new();

        let action = move_to_suspect(&mut migration);

        assert_eq!(action, PathHealthAction::None);
        assert_eq!(migration.state(), MigrationState::Suspect);
    }

    #[test]
    fn recovery_from_suspect_returns_controller_to_healthy() {
        let mut migration = MigrationController::new();

        move_to_suspect(&mut migration);

        let action = handle_path_health_event(
            PathHealthEvent::Recovered,
            RecoveryStrategy::Migrate,
            &mut migration,
            Duration::from_millis(350),
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap();

        assert_eq!(action, PathHealthAction::None);
        assert_eq!(migration.state(), MigrationState::Healthy);
    }

    #[test]
    fn persistent_degradation_requests_alternative_discovery() {
        let mut migration = MigrationController::new();

        let action = move_to_challenging(&mut migration);

        assert_eq!(
            action,
            PathHealthAction::RecoverUsing(RecoveryStrategy::Migrate)
        );
        assert_eq!(migration.state(), MigrationState::Challenging);
    }

    #[test]
    fn persistent_degradation_routes_to_reconnect_strategy() {
        let mut migration = MigrationController::new();

        handle_path_health_event(
            PathHealthEvent::BecameSuspect,
            RecoveryStrategy::Reconnect,
            &mut migration,
            Duration::from_millis(250),
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap();

        let action = handle_path_health_event(
            PathHealthEvent::ChallengeRequested,
            RecoveryStrategy::Reconnect,
            &mut migration,
            Duration::from_millis(750),
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap();

        assert_eq!(
            action,
            PathHealthAction::RecoverUsing(RecoveryStrategy::Reconnect)
        );
        assert_eq!(migration.state(), MigrationState::Challenging);
    }

    #[test]
    fn recovery_after_rebind_confirms_automatic_migration() {
        let mut migration = MigrationController::new();
        let old_local: SocketAddr = "10.0.1.2:5000".parse().unwrap();
        let new_local: SocketAddr = "10.0.2.2:6000".parse().unwrap();

        move_to_challenging(&mut migration);

        migration
            .transition(
                MigrationState::Migrating,
                MigrationReason::AlternatePathReady,
                MigrationContext {
                    elapsed: Duration::from_millis(800),
                    active_local: old_local,
                    candidate_local: Some(new_local),
                    connection_id: TEST_CONNECTION_ID,
                },
            )
            .unwrap();

        let action = handle_path_health_event(
            PathHealthEvent::Recovered,
            RecoveryStrategy::Migrate,
            &mut migration,
            Duration::from_millis(900),
            new_local,
            TEST_CONNECTION_ID,
        )
        .unwrap();

        assert_eq!(action, PathHealthAction::None);
        assert_eq!(migration.state(), MigrationState::Healthy);
    }

    #[test]
    fn recovery_event_while_healthy_is_ignored() {
        let mut migration = MigrationController::new();

        let action = handle_path_health_event(
            PathHealthEvent::Recovered,
            RecoveryStrategy::Migrate,
            &mut migration,
            Duration::from_millis(100),
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap();

        assert_eq!(action, PathHealthAction::None);
        assert_eq!(migration.state(), MigrationState::Healthy);
    }

    #[test]
    fn recovery_while_challenging_returns_controller_to_healthy() {
        let mut migration = MigrationController::new();

        move_to_challenging(&mut migration);

        let action = handle_path_health_event(
            PathHealthEvent::Recovered,
            RecoveryStrategy::Migrate,
            &mut migration,
            Duration::from_millis(800),
            test_local_address(),
            TEST_CONNECTION_ID,
        )
        .unwrap();

        assert_eq!(action, PathHealthAction::None);
        assert_eq!(migration.state(), MigrationState::Healthy);
    }

    #[test]
    fn recovery_after_automatic_rebind_completes_migration() {
        let mut migration = MigrationController::new();

        move_to_challenging(&mut migration);

        let old_local: SocketAddr = "10.0.1.2:5000".parse().unwrap();
        let new_local: SocketAddr = "10.0.2.2:6000".parse().unwrap();

        migration
            .transition(
                MigrationState::Migrating,
                MigrationReason::AlternatePathReady,
                MigrationContext {
                    elapsed: Duration::from_millis(800),
                    active_local: old_local,
                    candidate_local: Some(new_local),
                    connection_id: TEST_CONNECTION_ID,
                },
            )
            .unwrap();

        let action = handle_path_health_event(
            PathHealthEvent::Recovered,
            RecoveryStrategy::Migrate,
            &mut migration,
            Duration::from_millis(900),
            new_local,
            TEST_CONNECTION_ID,
        )
        .unwrap();

        assert_eq!(action, PathHealthAction::None);
        assert_eq!(migration.state(), MigrationState::Healthy);
    }
}
