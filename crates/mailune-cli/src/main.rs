//! Command-line surface. Dispatches into `mailune-app` and the scratch account file.
//!
//! Library crates keep their own error types. Only this binary uses `anyhow`.
//! Account commands read and write JSON under `MAILUNE_HOME` and do not open a socket.

mod account;

use std::io::{self, Write};

use anyhow::Context;
use clap::{Parser, Subcommand};

/// Local-first mail commands.
#[derive(Parser)]
#[command(name = "mailune", version, about = "Local-first mail")]
struct Cli {
    /// Subcommand.
    #[command(subcommand)]
    command: Command,
}

/// Top-level commands.
#[derive(Subcommand)]
enum Command {
    /// Accounts stored under `MAILUNE_HOME`.
    Account {
        /// Account action.
        #[command(subcommand)]
        action: AccountCommand,
    },
}

/// `account` actions.
#[derive(Subcommand)]
enum AccountCommand {
    /// Record an account in the scratch home. No network.
    Add {
        /// Mail address (`local@domain`).
        #[arg(long)]
        address: String,
        /// Display label. Empty when omitted.
        #[arg(long, default_value = "")]
        label: String,
    },
    /// Print stored accounts, one `address<TAB>label` line each.
    List,
}

fn main() -> anyhow::Result<()> {
    mailune_app::ready()?;
    let cli = Cli::parse();
    match cli.command {
        Command::Account { action } => run_account(action),
    }
}

fn run_account(action: AccountCommand) -> anyhow::Result<()> {
    let home = account::home_from_env()?;
    match action {
        AccountCommand::Add { address, label } => {
            account::add(&home, &address, &label)?;
            Ok(())
        }
        AccountCommand::List => {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            for item in account::list(&home)? {
                writeln!(out, "{}\t{}", item.address, item.label)
                    .context("writing the account list")?;
            }
            Ok(())
        }
    }
}
