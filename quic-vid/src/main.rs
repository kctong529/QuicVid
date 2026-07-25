mod client;
mod control;
mod frame_tracker;
mod media;
mod server;
mod tls;

use clap::{Parser, Subcommand};
use std::net::SocketAddr;

#[derive(Debug, Parser)]
#[command(name = "quic-vid")]
#[command(about = "QUIC video migration prototype")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the QuicVid server.
    Server {
        #[arg(long, default_value = "0.0.0.0:4433")]
        listen: SocketAddr,
    },

    /// Run the QuicVid client.
    Client {
        /// Server QUIC address.
        #[arg(long)]
        connect: SocketAddr,

        /// Local UDP address used by the Quinn endpoint.
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: SocketAddr,

        #[arg(long, default_value_t = 30)]
        fps: u32,

        #[arg(long, default_value_t = 10)]
        duration_seconds: u64,

        #[arg(long, default_value_t = 256)]
        payload_size: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls ring crypto provider");

    let cli = Cli::parse();

    match cli.command {
        Command::Server { listen } => server::run(listen).await,
        Command::Client {
            connect,
            bind,
            fps,
            duration_seconds,
            payload_size,
        } => client::run(connect, bind, fps, duration_seconds, payload_size).await,
    }
}
