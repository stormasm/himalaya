use anyhow::Result;
use dialoguer::{Confirm, Input, Select};
use shellexpand_utils::expand;
use std::{fs, path::PathBuf, process};
use toml_edit::{Document, Item};

use crate::{account, ui::THEME};

use super::TomlConfig;

#[macro_export]
macro_rules! wizard_warn {
    ($($arg:tt)*) => {
	println!("{}", console::style(format!($($arg)*)).yellow().bold());
    };
}

#[macro_export]
macro_rules! wizard_prompt {
    ($($arg:tt)*) => {
	format!("{}", console::style(format!($($arg)*)).italic())
    };
}

#[macro_export]
macro_rules! wizard_log {
    ($($arg:tt)*) => {
	println!();
	println!("{}", console::style(format!($($arg)*)).underlined());
	println!();
    };
}

pub(crate) async fn configure(path: PathBuf) -> Result<TomlConfig> {
    wizard_log!("Configuring your first account:");

    let mut config = TomlConfig::default();

    while let Some((name, account_config)) = account::wizard::configure().await? {
        config.accounts.insert(name, account_config);

        if !Confirm::new()
            .with_prompt(wizard_prompt!(
                "Would you like to configure another account?"
            ))
            .default(false)
            .interact_opt()?
            .unwrap_or_default()
        {
            break;
        }

        wizard_log!("Configuring another account:");
    }

    // If one account is setup, make it the default. If multiple
    // accounts are setup, decide which will be the default. If no
    // accounts are setup, exit the process.
    let default_account = match config.accounts.len() {
        0 => {
            wizard_warn!("No account configured, exiting.");
            process::exit(0);
        }
        1 => Some(config.accounts.values_mut().next().unwrap()),
        _ => {
            let accounts = config.accounts.clone();
            let accounts: Vec<&String> = accounts.keys().collect();

            println!("{} accounts have been configured.", accounts.len());

            Select::with_theme(&*THEME)
                .with_prompt(wizard_prompt!(
                    "Which account would you like to set as your default?"
                ))
                .items(&accounts)
                .default(0)
                .interact_opt()?
                .and_then(|idx| config.accounts.get_mut(accounts[idx]))
        }
    };

    if let Some(account) = default_account {
        account.default = Some(true);
    } else {
        process::exit(0)
    }

    let path = Input::with_theme(&*THEME)
        .with_prompt(wizard_prompt!(
            "Where would you like to save your configuration?"
        ))
        .default(path.to_string_lossy().to_string())
        .interact()?;
    let path = expand::path(path);

    println!("Writing the configuration to {path:?}…");

    let mut doc = toml::to_string(&config)?.parse::<Document>()?;

    doc.iter_mut().for_each(|(_, item)| {
        set_table_dotted(item, "folder-aliases");
        set_table_dotted(item, "sync-folders-strategy");

        set_table_dotted(item, "folder");
        if let Some(item) = get_table_mut(item, "folder") {
            set_tables_dotted(item, ["add", "list", "expunge", "purge", "delete"]);
        }

        set_table_dotted(item, "envelope");
        if let Some(item) = get_table_mut(item, "envelope") {
            set_tables_dotted(item, ["list", "get"]);
        }

        set_table_dotted(item, "flag");
        if let Some(item) = get_table_mut(item, "flag") {
            set_tables_dotted(item, ["add", "set", "remove"]);
        }

        set_table_dotted(item, "message");
        if let Some(item) = get_table_mut(item, "message") {
            set_tables_dotted(
                item,
                ["add", "send", "peek", "get", "copy", "move", "delete"],
            );
        }

        set_table_dotted(item, "maildir");
        #[cfg(feature = "imap")]
        {
            set_table_dotted(item, "imap");
            if let Some(item) = get_table_mut(item, "imap") {
                set_tables_dotted(item, ["passwd", "oauth2"]);
            }
        }
        #[cfg(feature = "notmuch")]
        set_table_dotted(item, "notmuch");
        set_table_dotted(item, "sendmail");
        #[cfg(feature = "smtp")]
        {
            set_table_dotted(item, "smtp");
            if let Some(item) = get_table_mut(item, "smtp") {
                set_tables_dotted(item, ["passwd", "oauth2"]);
            }
        }

        #[cfg(feature = "pgp")]
        set_table_dotted(item, "pgp");
    });

    fs::create_dir_all(path.parent().unwrap_or(&path))?;
    fs::write(path, doc.to_string())?;

    Ok(config)
}

fn get_table_mut<'a>(item: &'a mut Item, key: &'a str) -> Option<&'a mut Item> {
    item.get_mut(key).filter(|item| item.is_table())
}

fn set_table_dotted(item: &mut Item, key: &str) {
    if let Some(table) = get_table_mut(item, key).and_then(|item| item.as_table_mut()) {
        table.set_dotted(true)
    }
}

fn set_tables_dotted<'a>(item: &'a mut Item, keys: impl IntoIterator<Item = &'a str>) {
    for key in keys {
        set_table_dotted(item, key)
    }
}
