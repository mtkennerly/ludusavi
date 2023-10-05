use crate::prelude::Error;

/// GUI looks nicer with an extra empty line as separator, but for terminals a single
/// newline is sufficient
fn get_separator(gui: bool) -> &'static str {
    match gui {
        true => "\n\n",
        false => "\n",
    }
}

// ---------------------------------------------------------------------------
//
// Alerts
//
// ---------------------------------------------------------------------------

pub fn alert_with_error(gui: bool, msg: &str, error: &String) -> Result<(), Error> {
    alert(gui, &format!("{}{}Error message: {}", msg, get_separator(gui), error))
}

pub fn alert(gui: bool, msg: &str) -> Result<(), Error> {
    if gui {
        match native_dialog::MessageDialog::new()
            .set_title("Ludusavi Wrap Error")
            .set_text(msg)
            .set_type(native_dialog::MessageType::Error)
            .show_alert()
        {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::WrapCommandUITechnicalFailure { msg: err.to_string() }),
        }
    } else {
        // Using select is a hack since dialoguer does not have a

        // TODO.2023-10-05 bad style offering a choice when a OK is the only
        // option, but dialoguer does not have an alert type yet.
        //
        // Check this issue / pull request for progress:
        // https://github.com/console-rs/dialoguer/issues/287
        // https://github.com/console-rs/dialoguer/pull/288
        match dialoguer::Confirm::new().with_prompt(msg).default(true).interact() {
            Ok(_) => Ok(()),
            Err(err) => Err(Error::WrapCommandUITechnicalFailure { msg: err.to_string() }),
        }
    }
}

// ---------------------------------------------------------------------------
//
// Confirmations
//
// ---------------------------------------------------------------------------

pub fn confirm_continue(gui: bool, msg: &str) -> Result<bool, Error> {
    confirm_with_question(gui, msg, "Continue (YES) or abort (NO)?")
}

pub fn confirm_with_question(gui: bool, msg: &str, question: &str) -> Result<bool, Error> {
    confirm_simple(gui, &format!("{}{}{}", msg, get_separator(gui), question))
}

pub fn confirm_simple(gui: bool, msg: &str) -> Result<bool, Error> {
    if gui {
        match native_dialog::MessageDialog::new()
            .set_title("Ludusavi Wrap")
            .set_text(msg)
            .set_type(native_dialog::MessageType::Info)
            .show_confirm()
        {
            Ok(value) => Ok(value),
            Err(err) => Err(Error::WrapCommandUITechnicalFailure { msg: err.to_string() }),
        }
    } else {
        match dialoguer::Confirm::new().with_prompt(msg).interact() {
            Ok(value) => Ok(value),
            Err(err) => Err(Error::WrapCommandUITechnicalFailure { msg: err.to_string() }),
        }
    }
}
