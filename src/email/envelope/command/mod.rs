pub mod list;

use anyhow::Result;
use clap::Subcommand;

use crate::{config::TomlConfig, printer::Printer};

use self::list::EnvelopeListCommand;

/// Manage envelopes.
///
/// An envelope is a small representation of a message. It contains an
/// identifier (given by the backend), some flags as well as few
/// headers from the message itself. This subcommand allows you to
/// manage them.
#[derive(Debug, Subcommand)]
pub enum EnvelopeSubcommand {
    #[command(alias = "lst")]
    List(EnvelopeListCommand),
}

impl EnvelopeSubcommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.execute(printer, config).await,
        }
    }
}