mod client;
mod control;
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
        Command::Client { connect, bind } => client::run(connect, bind).await,
    }
}
