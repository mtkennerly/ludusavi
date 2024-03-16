use crate::{lang::TRANSLATOR, prelude::Error};

/// GUI looks nicer with an extra empty line as separator, but for terminals a single
/// newline is sufficient
fn get_separator(gui: bool) -> &'static str {
    match gui {
        true => "\n\n",
        false => "\n",
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

pub fn alert_with_raw_error(gui: bool, msg: &str, error: &str) -> Result<(), Error> {
    alert(
        gui,
        &format!("{}{}{}", msg, get_separator(gui), TRANSLATOR.prefix_error(error)),
    )
}

pub fn alert_with_error(gui: bool, msg: &str, error: &Error) -> Result<(), Error> {
    alert(
        gui,
        &format!("{}{}{}", msg, get_separator(gui), TRANSLATOR.handle_error(error)),
    )
}

pub fn alert(gui: bool, msg: &str) -> Result<(), Error> {
    log::debug!("Showing alert to user (GUI={}): {}", gui, msg);
    if gui {
        match native_dialog::MessageDialog::new()
            .set_title(&TRANSLATOR.app_name())
            .set_text(msg)
            .set_type(native_dialog::MessageType::Error)
            .show_alert()
        {
            Ok(_) => Ok(()),
            Err(err) => {
                log::error!("Unable to show alert: {:?}", err);
                Err(Error::CliUnableToRequestConfirmation)
            }
        }
    } else {
        // TODO: Dialoguer doesn't have an alert type.
        // https://github.com/console-rs/dialoguer/issues/287
        println!("{}", msg);
        pause()
    }
}

pub fn confirm_with_question(gui: bool, force: Option<bool>, msg: &str, question: &str) -> Result<bool, Error> {
    if let Some(force) = force {
        _ = alert(gui, msg);
        return Ok(force);
    }

    confirm(gui, None, &format!("{}{}{}", msg, get_separator(gui), question))
}

pub fn confirm(gui: bool, force: Option<bool>, msg: &str) -> Result<bool, Error> {
    log::debug!("Showing confirmation to user (GUI={}, force={:?}): {}", gui, force, msg);

    if let Some(force) = force {
        return Ok(force);
    }

    if gui {
        match native_dialog::MessageDialog::new()
            .set_title(&TRANSLATOR.app_name())
            .set_text(msg)
            .set_type(native_dialog::MessageType::Info)
            .show_confirm()
        {
            Ok(value) => {
                log::debug!("User responded: {}", value);
                Ok(value)
            }
            Err(err) => {
                log::error!("Unable to request confirmation: {:?}", err);
                Err(Error::CliUnableToRequestConfirmation)
            }
        }
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
