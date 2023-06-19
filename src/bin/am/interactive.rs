use std::io;
use dialoguer::theme::SimpleTheme;
use dialoguer::Input;

pub fn user_input(prompt: impl Into<String>) -> io::Result<String> {
    Ok(Input::with_theme(&SimpleTheme)
        .with_prompt(prompt)
        .interact()?)
}
