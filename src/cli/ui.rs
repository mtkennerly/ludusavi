use crate::{lang::TRANSLATOR, prelude::Error};

/// GUI looks nicer with an extra empty line as separator, but for terminals a single
/// newline is sufficient
fn get_separator(gui: bool) -> &'static str {
    match gui {
        true => "\n\n",
        false => "\n",
    }
}

/// Pad a string to a specific width
fn pad_to_width(text: &str, width: usize) -> String {
    if text.len() >= width {
        text.to_string()
    } else {
        format!("{}{}", text, " ".repeat(width - text.len()))
    }
}

    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    // Use TRANSLATOR for internationalization support
    write!(stdout, "{}", TRANSLATOR.press_any_key_to_continue()).map_err(|_| Error::CliUnableToRequestConfirmation)?;
    stdout.flush().map_err(|_| Error::CliUnableToRequestConfirmation)?;

    stdin
        .read(&mut [0u8])
        .map_err(|_| Error::CliUnableToRequestConfirmation)?;

    Ok(())

pub fn alert_with_raw_error(gui: bool, force: bool, msg: &str, error: &str) -> Result<(), Error> {
    alert(
        gui,
        force,
        &format!("{}{}{}", msg, get_separator(gui), TRANSLATOR.prefix_error(error)),
    )
}

pub fn alert_with_error(gui: bool, force: bool, msg: &str, error: &Error) -> Result<(), Error> {
    alert(
        gui,
        force,
        &format!("{}{}{}", msg, get_separator(gui), TRANSLATOR.handle_error(error)),
    )
}

pub fn alert(gui: bool, force: bool, msg: &str) -> Result<(), Error> {
    log::debug!("Showing alert to user (GUI={}, force={}): {}", gui, force, msg);
    if gui {
        rfd::MessageDialog::new()
            .set_title(TRANSLATOR.app_name())
            .set_description(msg)
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
        Ok(())
    } else if !force {
        // Dialoguer doesn't have a built-in alert type yet
        // Tracking issue: https://github.com/console-rs/dialoguer/issues/287
        // For now, we create our own alert-like display
        println!("\n┌─────────────────────────────────────────┐");
        println!("│ ⚠️  Alert                                │");
        println!("├─────────────────────────────────────────┤");
        println!("│ {}│", format!(" {}", msg).pad_to_width(39));
        println!("└─────────────────────────────────────────┘\n");
        pause()
    } else {
        println!("{}", msg);
        Ok(())
    }
}

pub fn confirm_with_question(gui: bool, force: bool, preview: bool, msg: &str, question: &str) -> Result<bool, Error> {
    if force || preview {
        _ = alert(gui, force, msg);
        return Ok(true);
    }

    confirm(
        gui,
        force,
        preview,
        &format!("{}{}{}", msg, get_separator(gui), question),
    )
}

pub fn confirm(gui: bool, force: bool, preview: bool, msg: &str) -> Result<bool, Error> {
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
            .set_title(TRANSLATOR.app_name())
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
