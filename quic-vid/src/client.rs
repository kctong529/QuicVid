use crate::{control, tls};
use std::net::SocketAddr;
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

    connection.close(0u32.into(), b"client done");
    endpoint.wait_idle().await;

    println!("event=client_stopped session={session_id}");

    Ok(())
}
