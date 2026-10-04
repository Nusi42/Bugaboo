//! The `bugaboo` binary: command line, setup wizard, service install and wiring of all adapters.

#![forbid(unsafe_code)]

mod output;

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Context as _;
use bb_api::WebSource;
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;

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
    // Local only until TLS and sign-in exist (M1).
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let listener = TcpListener::bind(address).await.with_context(|| {
        format!("Port {port} is busy or not allowed. Change it in tools/dev/ports.json.")
    })?;
    if !source.has_index().await {
        output::warning(bb_api::WEB_APP_MISSING);
    }
    output::line(&format!("Listening on http://{address}"));
    bb_api::serve(listener, source, shutdown_signal()).await?;
    Ok(())
}

async fn shutdown_signal() {
    // If the signal handler cannot be installed there is nothing to wait for; stop at once.
    let _ = tokio::signal::ctrl_c().await;
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
