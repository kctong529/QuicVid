mod client;
mod control;
mod frame_assembler;
mod frame_tracker;
mod media;
mod migration;
mod path_health;
mod preview;
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

        /// Show received video in a live preview window.
        #[arg(long)]
        preview: bool,
    },

    /// Run the QuicVid client.
    Client {
        /// Server QUIC address.
        #[arg(long)]
        connect: SocketAddr,

        /// Local UDP address used by the Quinn endpoint.
        #[arg(long, default_value = "0.0.0.0:0")]
        bind: SocketAddr,

        #[arg(long)]
        rebind: Option<SocketAddr>,

        #[arg(long)]
        rebind_after_seconds: Option<f64>,

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

    /// Open one generated test frame in the preview window.
    PreviewTestFrame {
        /// Logical frame ID to preview.
        #[arg(long, default_value_t = 42)]
        frame_id: u64,

        /// JPEG quality from 1 to 100.
        #[arg(
            long,
            default_value_t = test_pattern::DEFAULT_JPEG_QUALITY
        )]
        quality: u8,
    },

    /// Show an animated generated test stream in the preview window.
    PreviewTestStream {
        /// Frames generated per second.
        #[arg(long, default_value_t = 10)]
        fps: u32,

        /// Number of seconds to generate.
        #[arg(long, default_value_t = 10)]
        duration_seconds: u64,

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
        Command::Server { listen, preview } => {
            if preview {
                run_server_with_preview(listen)
            } else {
                server::run(listen, None).await
            }
        }

        Command::Client {
            connect,
            bind,
            rebind,
            rebind_after_seconds,
            fps,
            duration_seconds,
            jpeg_quality,
        } => {
            client::run(
                connect,
                bind,
                rebind,
                rebind_after_seconds,
                fps,
                duration_seconds,
                jpeg_quality,
            )
            .await
        }

        Command::GenerateTestFrames {
            count,
            output,
            quality,
        } => {
            test_pattern::write_preview_frames(&output, count, quality)?;
            Ok(())
        }

        Command::PreviewTestFrame { frame_id, quality } => {
            let jpeg = test_pattern::generate_jpeg_frame(frame_id, quality)?;
            let frame = preview::preview_frame_from_jpeg(frame_id, &jpeg)?;

            preview::show_preview_frame(&frame)?;

            Ok(())
        }

        Command::PreviewTestStream {
            fps,
            duration_seconds,
            quality,
        } => run_preview_test_stream(fps, duration_seconds, quality),
    }
}

fn run_preview_test_stream(fps: u32, duration_seconds: u64, quality: u8) -> anyhow::Result<()> {
    if fps == 0 {
        anyhow::bail!("fps must be greater than zero");
    }

    if duration_seconds == 0 {
        anyhow::bail!("duration-seconds must be greater than zero");
    }

    if !(1..=100).contains(&quality) {
        anyhow::bail!("JPEG quality must be between 1 and 100");
    }

    let total_frames = u64::from(fps)
        .checked_mul(duration_seconds)
        .ok_or_else(|| anyhow::anyhow!("preview frame count overflow"))?;

    let (sender, receiver) = preview::channel();

    let producer = std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let frame_interval = 1.0 / f64::from(fps);

        let mut last_frame_id = None;

        loop {
            let elapsed = start.elapsed().as_secs_f64();

            let frame_id = (elapsed / frame_interval).floor() as u64;

            if frame_id >= total_frames {
                break;
            }

            if last_frame_id == Some(frame_id) {
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }

            last_frame_id = Some(frame_id);

            let jpeg = test_pattern::generate_jpeg_frame(frame_id, quality)?;

            let preview_jpeg = preview::PreviewJpeg {
                frame_id,
                bytes: jpeg,
            };

            if preview::publish(&sender, preview_jpeg).is_err() {
                break;
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    let preview_result = preview::show_preview_stream(receiver);

    producer
        .join()
        .map_err(|_| anyhow::anyhow!("preview producer thread panicked"))??;

    preview_result
}

fn run_server_with_preview(listen: SocketAddr) -> anyhow::Result<()> {
    let (preview_sender, preview_receiver) = preview::channel();

    let server_thread = std::thread::spawn(move || -> anyhow::Result<()> {
        let runtime = tokio::runtime::Runtime::new()?;

        runtime.block_on(server::run(listen, Some(preview_sender)))
    });

    let preview_result = preview::show_preview_stream(preview_receiver);

    let server_result = server_thread
        .join()
        .map_err(|_| anyhow::anyhow!("server thread panicked"))?;

    preview_result?;
    server_result
}
