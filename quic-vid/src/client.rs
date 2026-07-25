use std::net::SocketAddr;

pub async fn run(connect: SocketAddr, bind: SocketAddr) -> anyhow::Result<()> {
    println!("client connect={connect} bind={bind}");
    Ok(())
}
