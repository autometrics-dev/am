use anyhow::Result;
use itertools::Itertools;
use std::collections::HashMap;
use std::io::Write;
use std::time::Duration;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use tokio::sync::watch::Receiver;
use tracing::info;

pub(crate) fn wait_and_print_urls(mut rx: Receiver<HashMap<&'static str, String>>) {
    tokio::spawn(async move {
        // wait a second until all other log messages (invoked in belows `select!`) are printed
        // Prometheus and Pushgateway usually dont take longer than a second to start so this should be good
        tokio::time::sleep(Duration::from_secs(1)).await;

        match rx.wait_for(|map| !map.is_empty()).await {
            Ok(map) => {
                let _ = print_urls(&map);
            }
            Err(err) => {
                info!(?err, "failed to wait for urls");
            }
        }
    });
}

pub(crate) fn print_urls(map: &HashMap<&str, String>) -> Result<()> {
    let length = map
        .iter()
        .map(|(name, _)| name.len() + 5)
        .max()
        .unwrap_or(0);

    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Magenta)).set_bold(true))?;
    write!(stdout, "\n  am ")?;

    stdout.set_color(
        ColorSpec::new()
            .set_fg(Some(Color::Magenta))
            .set_bold(false),
    )?;
    write!(stdout, "v{}", env!("CARGO_PKG_VERSION"))?;

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;
    writeln!(stdout, "   press ctrl + c to shutdown\n")?;

    for (name, url) in map.iter().sorted_by(|(a, _), (b, _)| a.cmp(b)) {
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(true))?;
        write!(stdout, "  {:width$}", name, width = length)?;

        stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false))?;
        writeln!(stdout, "  {}", url)?;
    }

    writeln!(stdout, "")?;
    Ok(())
}
