use crate::{
    lang::TRANSLATOR,
    prelude::{Error, SyncDirection},
};

/// GUI looks nicer with an extra empty line as separator, but for terminals a single
/// newline is sufficient
fn get_separator(gui: bool) -> &'static str {
    match gui {
        true => "\n\n",
        false => "\n",
    }
}

fn title(games: &[String]) -> String {
    match games.len() {
        0 => TRANSLATOR.app_name(),
        1 => format!("{} - {}", TRANSLATOR.app_name(), &games[0]),
        total => format!("{} - {}: {}", TRANSLATOR.app_name(), TRANSLATOR.total_games(), total),
    }
}

fn pause() -> Result<(), Error> {
    use std::io::prelude::{Read, Write};

    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    // TODO: Must be a string literal. Can we support translation?
    write!(stdout, "Press any key to continue...").map_err(|_| Error::CliUnableToRequestConfirmation)?;
    stdout.flush().map_err(|_| Error::CliUnableToRequestConfirmation)?;

    stdin
        .read(&mut [0u8])
        .map_err(|_| Error::CliUnableToRequestConfirmation)?;

    Ok(())
}

pub fn alert_with_raw_error(games: &[String], gui: bool, force: bool, msg: &str, error: &str) -> Result<(), Error> {
    alert(
        games,
        gui,
        force,
        &format!("{}{}{}", msg, get_separator(gui), TRANSLATOR.prefix_error(error)),
    )
}

pub fn alert_with_error(games: &[String], gui: bool, force: bool, msg: &str, error: &Error) -> Result<(), Error> {
    alert(
        games,
        gui,
        force,
        &format!("{}{}{}", msg, get_separator(gui), TRANSLATOR.handle_error(error)),
    )
}

pub fn alert(games: &[String], gui: bool, force: bool, msg: &str) -> Result<(), Error> {
    log::debug!("Showing alert to user (GUI={}, force={}): {}", gui, force, msg);
    if gui {
        rfd::MessageDialog::new()
            .set_title(title(games))
            .set_description(msg)
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
        Ok(())
    } else if !force {
        // TODO: Dialoguer doesn't have an alert type.
        // https://github.com/console-rs/dialoguer/issues/287
        println!("{msg}");
        pause()
    } else {
        println!("{msg}");
        Ok(())
    }
}

pub fn confirm_with_question(
    games: &[String],
    gui: bool,
    force: bool,
    preview: bool,
    msg: &str,
    question: &str,
) -> Result<bool, Error> {
    if force || preview {
        _ = alert(games, gui, force, msg);
        return Ok(true);
    }

    confirm(
        games,
        gui,
        force,
        preview,
        &format!("{}{}{}", msg, get_separator(gui), question),
    )
}

pub fn confirm(games: &[String], gui: bool, force: bool, preview: bool, msg: &str) -> Result<bool, Error> {
    log::debug!(
        "Showing confirmation to user (GUI={}, force={}, preview={}): {}",
        gui,
        force,
        preview,
        msg
    );

    if force || preview {
        return Ok(true);
    }

    if gui {
        let choice = match rfd::MessageDialog::new()
            .set_title(title(games))
            .set_description(msg)
            .set_level(rfd::MessageLevel::Info)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show()
        {
            rfd::MessageDialogResult::Yes => true,
            rfd::MessageDialogResult::No => false,
            rfd::MessageDialogResult::Ok => true,
            rfd::MessageDialogResult::Cancel => false,
            rfd::MessageDialogResult::Custom(_) => false,
        };
        log::debug!("User responded: {}", choice);
        Ok(choice)
    } else {
        match dialoguer::Confirm::new().with_prompt(msg).interact() {
            Ok(value) => {
                log::debug!("User responded: {}", value);
                Ok(value)
            }
            Err(err) => {
                log::error!("Unable to request confirmation: {:?}", err);
                Err(Error::CliUnableToRequestConfirmation)
            }
        }
    }
}

pub fn ask_cloud_conflict(
    games: &[String],
    gui: bool,
    force: bool,
    preview: bool,
) -> Result<Option<SyncDirection>, Error> {
    let msg = TRANSLATOR.cloud_synchronize_conflict();

    log::debug!(
        "Asking user about cloud conflict (GUI={}, force={}, preview={}): {}",
        gui,
        force,
        preview,
        msg,
    );

    if force || preview {
        return Ok(None);
    }

    fn parse_response(raw: &str) -> Option<SyncDirection> {
        if raw == TRANSLATOR.download_button() {
            Some(SyncDirection::Download)
        } else if raw == TRANSLATOR.upload_button() {
            Some(SyncDirection::Upload)
        } else {
            None
        }
    }

    if gui {
        let choice = match rfd::MessageDialog::new()
            .set_title(title(games))
            .set_description(msg)
            .set_level(rfd::MessageLevel::Info)
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                TRANSLATOR.ignore_button(),
                TRANSLATOR.download_button(),
                TRANSLATOR.upload_button(),
            ))
            .show()
        {
            rfd::MessageDialogResult::Yes => None,
            rfd::MessageDialogResult::No => None,
            rfd::MessageDialogResult::Ok => None,
            rfd::MessageDialogResult::Cancel => None,
            rfd::MessageDialogResult::Custom(raw) => parse_response(&raw),
        };
        log::debug!("User responded: {:?}", choice);
        Ok(choice)
    } else {
        let options = vec![
            TRANSLATOR.ignore_button(),
            TRANSLATOR.download_button(),
            TRANSLATOR.upload_button(),
        ];

        let dialog = dialoguer::Select::new().with_prompt(msg).items(&options);

        match dialog.interact() {
            Ok(index) => {
                let choice = parse_response(&options[index]);
                log::debug!("User responded: {} -> {:?}", index, choice);
                Ok(choice)
            }
            Err(err) => {
                log::error!("Unable to request confirmation: {:?}", err);
                Err(Error::CliUnableToRequestConfirmation)
            }
        }
    }
}
