use std::net::SocketAddr;

pub async fn run(listen: SocketAddr) -> anyhow::Result<()> {
    println!("server listen={listen}");
    Ok(())
}
