use clap::{Args, Parser, Subcommand};
use directories::BaseDirs;
use rustyline::{error::ReadlineError, Editor};
use serin_parser::parse;
use std::{fs, path::PathBuf};

/// SerinDB command-line client.
#[derive(Parser)]
#[command(name = "serinctl", author, version, about = "SerinDB CLI Tool", long_about = None)]
struct Cli {
    /// Execute SQL directly and exit.
    #[arg(short = 'e', long = "exec")]
    sql: Option<String>,

    /// Execute SQL file and exit.
    #[arg(short = 'f', long = "file")]
    file: Option<PathBuf>,

    #[command(flatten)]
    opts: Options,
}

#[derive(Args, Default)]
struct Options {
    /// Path to configuration file (default: $HOME/.serinrc).
    #[arg(long = "config")]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Shard management commands.
    Shard {
        #[arg(long)]
        key: String,
        #[arg(long, default_value_t = 4)]
        shards: u64,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_path = cli
        .opts
        .config
        .or_else(|| BaseDirs::new().map(|b| b.home_dir().join(".serinrc")));

    if let Some(cfg) = config_path {
        if cfg.exists() {
            println!("Loaded config from {}", cfg.display());
        }
    }

    if let Some(sql) = cli.sql {
        execute_sql(&sql);
        return Ok(());
    }

    if let Some(file) = cli.file {
        let content = fs::read_to_string(file)?;
        for stmt in content.split(';') {
            if !stmt.trim().is_empty() {
                execute_sql(&(stmt.to_owned() + ";"));
            }
        }
        return Ok(());
    }

    match cli.command {
        Some(Commands::Shard { key, shards }) => {
            let router = serin_shard::HashRouter::new(shards);
            let rt = tokio::runtime::Runtime::new()?;
            let id = rt.block_on(router.shard_for_key(&key));
            println!("shard_id={}", id);
        }
        None => {}
    }

    interactive_shell();
    Ok(())
}

/// Evaluate a single SQL string and print the parsed AST or error.
fn execute_sql(sql: &str) {
    match parse(sql) {
        Ok(ast) => println!("{:#?}", ast),
        Err(e) => eprintln!("Error: {e}")
    }
}

/// Interactive readline shell.
fn interactive_shell() {
    let mut rl: Editor<()> = Editor::new().expect("failed to init editor");
    let prompt = "serinctl> ";

    loop {
        match rl.readline(prompt) {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq("\\q") {
                    break;
                }
                if trimmed.is_empty() {
                    continue;
                }
                rl.add_history_entry(trimmed);
                let sql = if trimmed.ends_with(';') {
                    trimmed.to_string()
                } else {
                    format!("{};", trimmed)
                };
                execute_sql(&sql);
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("Readline error: {err}");
                break;
            }
        }
    }
} 