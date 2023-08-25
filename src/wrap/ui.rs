pub fn alert_with_error(gui: bool, msg: &str, error: &String) -> Result<(), native_dialog::Error> {
    alert(gui, &format!("{}\n\nError message: {}", msg, error))
}

pub fn alert(_gui: bool, msg: &str) -> Result<(), native_dialog::Error> {
    native_dialog::MessageDialog::new()
        .set_title("Ludusavi Wrap Error")
        .set_text(msg)
        .set_type(native_dialog::MessageType::Error)
        .show_alert()
}

pub fn confirm_continue(gui: bool, msg: &str) -> Result<bool, native_dialog::Error> {
    confirm(gui, msg, "Continue (YES) or abort (NO)?")
}

pub fn confirm(gui: bool, msg: &str, question: &str) -> Result<bool, native_dialog::Error> {
    confirm_simple(gui, &format!("{}\n\n{}", msg, question))
}

pub fn confirm_simple(_gui: bool, msg: &str) -> Result<bool, native_dialog::Error> {
    native_dialog::MessageDialog::new()
        .set_title("Ludusavi Wrap")
        .set_text(msg)
        .set_type(native_dialog::MessageType::Info)
        .show_confirm()
}
