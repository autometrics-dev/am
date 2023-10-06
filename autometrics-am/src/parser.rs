use anyhow::{bail, Context, Result};
use url::Url;

/// Parses the input string into a Url. This uses a custom parser to allow for
/// some more flexible input.
///
/// Parsing adheres to the following rules:
/// - The protocol should only allow for http and https, where http is the
///   default.
/// - The port should follow the default for the protocol, 80 for http and 443
///   for https.
/// - The path should default to /metrics if the path is empty. It should not be
///   appended if a path is already there.
pub fn endpoint_parser(input: &str) -> Result<Url> {
    let mut input = input.to_owned();

    if input.starts_with(':') {
        // Prepend http://localhost if the input starts with a colon.
        input = format!("http://localhost{}", input);
    }

    // Prepend http:// if the input does not contain ://. This is a rather naive
    // check, but it should suffice for our purposes.
    if !input.contains("://") {
        input = format!("http://{}", input);
    }

    let mut url =
        Url::parse(&input).with_context(|| format!("Unable to parse endpoint {}", input))?;

    //  Note that this should never be Err(_) since we're always adding http://
    // in front of the input and thus making sure it is not a "cannot-be-a-base"
    // URL.
    if url.path() == "" || url.path() == "/" {
        url.set_path("/metrics");
    }

    if url.scheme() != "http" && url.scheme() != "https" {
        bail!("unsupported protocol {}", url.scheme());
    }

    Ok(url)
}
