use crate::media::MediaDatagram;
use crate::{control, tls};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub async fn run(connect: SocketAddr, bind: SocketAddr) -> anyhow::Result<()> {
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

    println!(
        "event=datagram_transport_ready session={} max_datagram_size={}",
        session_id, max_datagram_size,
    );

    let media = MediaDatagram {
        session_id,
        frame_id: 0,
        sent_at_ms: unix_time_ms()?,
        chunk_index: 0,
        chunk_count: 1,
        payload: vec![0x42; 32],
    };

    let encoded = media.encode()?;

    if encoded.len() > max_datagram_size {
        anyhow::bail!(
            "media datagram is too large: {} bytes, current maximum is {}",
            encoded.len(),
            max_datagram_size
        );
    }

    connection.send_datagram(encoded.into())?;

    println!(
        "event=media_datagram_sent session={} frame={} chunk={}/{} payload_bytes={}",
        session_id,
        media.frame_id,
        media.chunk_index,
        media.chunk_count,
        media.payload.len(),
    );

    let close_reason = connection.closed().await;

    println!(
        "event=server_closed_connection session={} reason={}",
        session_id, close_reason,
    );

    endpoint.wait_idle().await;

    println!("event=client_stopped session={session_id}");

    Ok(())
}

fn unix_time_ms() -> anyhow::Result<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;

    Ok(elapsed.as_millis().try_into()?)
}
