use clap::{Parser, Subcommand};
use tokio::runtime::Runtime;

/// SerinDB command-line interface (MVP).
#[derive(Parser)]
#[command(name = "serindb", author, version, about = "SerinDB CLI", long_about = None)]
struct Cli {
    /// Subcommands placeholder
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run health check and exit.
    HealthCheck,

    /// Start PostgreSQL Wire server.
    Server {
        /// Listen address (e.g., 0.0.0.0:5432)
        #[arg(long, default_value = "0.0.0.0:5432")]
        listen: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::HealthCheck) => {
            if serindb::health_check() {
                println!("OK");
            } else {
                println!("FAILED");
            }
        }
        Some(Commands::Server { listen }) => {
            // Start async runtime manually since main is sync.
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                if let Err(e) = serin_pgwire::run_server(&listen).await {
                    eprintln!("Server error: {e}");
                }
            });
        }
        None => {
            // Clap will print help.
        }
    }
} 