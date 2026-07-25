use crate::tls;
use std::net::SocketAddr;

pub async fn run(connect: SocketAddr, bind: SocketAddr) -> anyhow::Result<()> {
    let mut endpoint = quinn::Endpoint::client(bind)?;
    endpoint.set_default_client_config(tls::client_config()?);

    let local_addr = endpoint.local_addr()?;

    println!(
        "event=client_endpoint_created bind={} local={}",
        bind, local_addr
    );

    println!("event=connecting remote={connect}");

    let connection = endpoint.connect(connect, "localhost")?.await?;

    println!(
        "event=connected connection={} local={} remote={}",
        connection.stable_id(),
        endpoint.local_addr()?,
        connection.remote_address(),
    );

    connection.close(0u32.into(), b"client done");
    endpoint.wait_idle().await;

    println!("event=client_stopped");

    Ok(())
}
