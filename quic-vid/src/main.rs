mod client;
mod control;
mod frame_assembler;
mod frame_tracker;
mod media;
mod server;
mod test_pattern;
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

        #[arg(long, default_value_t = 10)]
        fps: u32,

        #[arg(long, default_value_t = 10)]
        duration_seconds: u64,

        #[arg(
            long,
            default_value_t = test_pattern::DEFAULT_JPEG_QUALITY
        )]
        jpeg_quality: u8,
    },

    /// Generate JPEG test-pattern frames for inspection.
    GenerateTestFrames {
        /// Number of frames to generate.
        #[arg(long, default_value_t = 3)]
        count: u64,

        /// Output directory for generated JPEG frames.
        #[arg(long, default_value = "test-frames")]
        output: std::path::PathBuf,

        /// JPEG quality from 1 to 100.
        #[arg(
            long,
            default_value_t = test_pattern::DEFAULT_JPEG_QUALITY
        )]
        quality: u8,
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
            jpeg_quality,
        } => client::run(connect, bind, fps, duration_seconds, jpeg_quality).await,

        Command::GenerateTestFrames {
            count,
            output,
            quality,
        } => {
            test_pattern::write_preview_frames(&output, count, quality)?;

            Ok(())
        }
    }
}
