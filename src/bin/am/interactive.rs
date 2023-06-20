use dialoguer::theme::SimpleTheme;
use dialoguer::{Confirm, Input};
use std::io;

pub fn user_input(prompt: impl Into<String>) -> io::Result<String> {
    Ok(Input::with_theme(&SimpleTheme)
        .with_prompt(prompt)
        .interact()?)
}

pub fn confirm(prompt: impl Into<String>) -> io::Result<bool> {
    Ok(Confirm::with_theme(&SimpleTheme)
        .with_prompt(prompt)
        .interact()?)
}
