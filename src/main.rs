use clap::Parser;

mod protocol;
mod server;
mod platform;
mod error;
mod command;
mod client;
mod stubs;

#[cfg(feature = "http")]
mod http_server;

mod tools;
mod resources;
mod prompts;

#[derive(Parser)]
#[command(name = "arch-opsd", version = "0.1.0", about = "MCP server for Arch Linux")]
enum Cli {
    Stdio,
    #[cfg(feature = "http")]
    Http {
        #[arg(default_value = "0.0.0.0:8080")]
        address: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli {
        Cli::Stdio => server::run_stdio().await?,
        #[cfg(feature = "http")]
        Cli::Http { address } => http_server::run_http(&address).await?,
    }
    Ok(())
}
