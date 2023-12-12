use anyhow::Result;
use clap::Parser;
use email::flag::Flag;
use log::info;
use mml::MmlCompilerBuilder;
use std::io::{self, BufRead, IsTerminal};

use crate::{
    account::arg::name::AccountNameFlag, backend::Backend, cache::arg::disable::CacheDisableFlag,
    config::TomlConfig, email::template::arg::TemplateRawArg, printer::Printer,
};

/// Send a template.
///
/// This command allows you to send a template and save a copy to the
/// sent folder. The template is compiled into a MIME message before
/// being sent. If you want to send a raw message, use the message
/// send command instead.
#[derive(Debug, Parser)]
pub struct TemplateSendCommand {
    #[command(flatten)]
    pub template: TemplateRawArg,

    #[command(flatten)]
    pub cache: CacheDisableFlag,

    #[command(flatten)]
    pub account: AccountNameFlag,
}

impl TemplateSendCommand {
    pub async fn execute(self, printer: &mut impl Printer, config: &TomlConfig) -> Result<()> {
        info!("executing template send command");

        let account = self.account.name.as_ref().map(String::as_str);
        let cache = self.cache.disable;

        let (toml_account_config, account_config) =
            config.clone().into_account_configs(account, cache)?;
        let backend = Backend::new(toml_account_config, account_config.clone(), true).await?;
        let folder = account_config.get_sent_folder_alias()?;

        let is_tty = io::stdin().is_terminal();
        let is_json = printer.is_json();
        let tpl = if is_tty || is_json {
            self.template.raw()
        } else {
            io::stdin()
                .lock()
                .lines()
                .filter_map(Result::ok)
                .collect::<Vec<String>>()
                .join("\r\n")
        };

        #[allow(unused_mut)]
        let mut compiler = MmlCompilerBuilder::new();

        #[cfg(feature = "pgp")]
        compiler.set_some_pgp(account_config.pgp.clone());

        let msg = compiler.build(tpl.as_str())?.compile().await?.into_vec()?;

        backend.send_raw_message(&msg).await?;

        if account_config.should_save_copy_sent_message() {
            backend
                .add_raw_message_with_flag(&folder, &msg, Flag::Seen)
                .await?;

            printer.print(format!("Template successfully sent and saved to {folder}!"))
        } else {
            printer.print("Template successfully sent!")
        }
    }
}