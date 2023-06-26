use dialoguer::theme::SimpleTheme;
use dialoguer::{Confirm, Input};
use indicatif::MultiProgress;
use std::io::{stderr, IoSlice, Result, Write};
use tracing_subscriber::fmt::MakeWriter;

pub fn user_input(prompt: impl Into<String>) -> Result<String> {
    Ok(Input::with_theme(&SimpleTheme)
        .with_prompt(prompt)
        .interact()?)
}

pub fn confirm(prompt: impl Into<String>) -> Result<bool> {
    Ok(Confirm::with_theme(&SimpleTheme)
        .with_prompt(prompt)
        .interact()?)
}

/// A Writer that will suspend any progress bar during calls to the write trait.
/// This will prevent the output from being mangled.
///
/// The main use case for this is to use it in conjunction with other components
/// that write to stderr, such as the tracing library. If both indicatif and
/// tracing would be using stderr directly, it would result in progress bars
/// being interrupted by other output.
#[derive(Clone)]
pub struct IndicatifWriter {
    multi_progress: indicatif::MultiProgress,
}

impl IndicatifWriter {
    /// Create a new IndicatifWriter. Make sure to use the returned
    /// MultiProgress when creating any progress bars.
    pub fn new() -> (Self, MultiProgress) {
        let multi_progress = MultiProgress::new();
        (
            Self {
                multi_progress: multi_progress.clone(),
            },
            multi_progress,
        )
    }
}

impl Write for IndicatifWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.multi_progress.suspend(|| stderr().write(buf))
    }

    fn flush(&mut self) -> Result<()> {
        self.multi_progress.suspend(|| stderr().flush())
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.multi_progress
            .suspend(|| stderr().write_vectored(bufs))
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.multi_progress.suspend(|| stderr().write_all(buf))
    }

    fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>) -> Result<()> {
        self.multi_progress.suspend(|| stderr().write_fmt(fmt))
    }
}

impl<'a> MakeWriter<'a> for IndicatifWriter {
    type Writer = IndicatifWriter;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}