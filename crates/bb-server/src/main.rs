//! The `bugaboo` binary: command line, setup wizard, service install and wiring of all adapters.

#![forbid(unsafe_code)]

mod output;

use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use bb_api::WebSource;
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;

/// Shown when the operating system will not let us listen for stop signals.
const SIGNAL_HANDLER_FAILED: &str =
    "Could not listen for the stop signal (Ctrl+C), so the server cannot be stopped cleanly.";

#[derive(Parser)]
#[command(name = "bugaboo", about = "Bugaboo: the issue tracker in one file")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the server.
    Serve {
        /// The port to listen on. There is deliberately no default (ADR-0023).
        #[arg(long)]
        port: u16,
        /// Serve the browser app from this folder instead of the one built into the program.
        #[arg(long)]
        web: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Serve { port, web } => serve(port, web).await,
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            output::error(&format!("{error:#}"));
            ExitCode::FAILURE
        }
    }
}

async fn serve(port: u16, web: Option<PathBuf>) -> anyhow::Result<()> {
    let source = web.map_or(WebSource::Embedded, WebSource::Folder);
    // Installed before listening, so a failure is reported instead of stopping silently.
    let shutdown = shutdown_signal()?;
    // Local only until TLS and sign-in exist (M1).
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("Port {port} is busy or not allowed."))?;
    if !source.has_index().await {
        output::warning(bb_api::WEB_APP_MISSING);
    }
    output::line(&format!("Listening on http://{address}"));
    bb_api::serve(listener, source, shutdown).await?;
    Ok(())
}

/// Completes on Ctrl+C or, on Unix, on SIGTERM (what service managers send).
fn shutdown_signal() -> anyhow::Result<impl Future<Output = ()> + Send + 'static> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut interrupt = signal(SignalKind::interrupt()).context(SIGNAL_HANDLER_FAILED)?;
        let mut terminate = signal(SignalKind::terminate()).context(SIGNAL_HANDLER_FAILED)?;
        Ok(async move {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = terminate.recv() => {}
            }
        })
    }
    #[cfg(windows)]
    {
        let mut interrupt = tokio::signal::windows::ctrl_c().context(SIGNAL_HANDLER_FAILED)?;
        Ok(async move {
            interrupt.recv().await;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dist_01_serve_needs_an_explicit_port() {
        assert!(Cli::try_parse_from(["bugaboo", "serve"]).is_err());
        assert!(Cli::try_parse_from(["bugaboo", "serve", "--port", "47200"]).is_ok());
        assert!(
            Cli::try_parse_from(["bugaboo", "serve", "--port", "47200", "--web", "some/dir"])
                .is_ok()
        );
    }
}
