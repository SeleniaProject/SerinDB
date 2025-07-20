use clap::{Parser, Subcommand};

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
        None => {
            // Default behavior: print help (handled by clap auto) if no subcommand
        }
    }
} 