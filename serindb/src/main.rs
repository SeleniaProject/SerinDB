use clap::{Parser, Subcommand};
use tokio::runtime::Runtime;
use serin_pgwire::auth::AuthConfig;

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

        /// Path to YAML auth file.
        #[arg(long, default_value = "serin_auth.yml")]
        auth_file: String,
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
        Some(Commands::Server { listen, auth_file }) => {
            // Start async runtime manually since main is sync.
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let conf = AuthConfig::load(&auth_file).expect("failed to load auth config");
                if let Err(e) = serin_pgwire::run_server(&listen, conf).await {
                    eprintln!("Server error: {e}");
                }
            });
        }
        None => {
            // Clap will print help.
        }
    }
} 