//! Desktop shell for Contexpect.
//!
//! The shell is a window around the UI that the local daemon already serves.
//! It exposes **no Tauri commands**: the UI talks to the daemon over the
//! loopback HTTP API it was built against, so there is no IPC surface here to
//! secure, and no path by which page content could reach the filesystem
//! through this process (ADR 0002).
//!
//! The daemon is started by the user, not by this shell. Spawning it would
//! mean shipping a process-execution capability whose only purpose is
//! convenience, and the address would still have to be discovered; asking for
//! the address keeps the trust boundary where the ADR puts it.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Url, WebviewUrl, WebviewWindowBuilder};

/// Where the daemon serves the UI.
///
/// Overridden with `CONTEXPECT_DAEMON_URL`. Only loopback origins are
/// accepted: a shell that would load an arbitrary URL is a browser, and this
/// window is not sandboxed like one.
fn daemon_url() -> Result<Url, String> {
    let raw = std::env::var("CONTEXPECT_DAEMON_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:7420".to_string());
    // The URL is parsed once, by the same parser the WebView will use, and
    // every check reads that parse: a hand-rolled split would accept
    // `http://127.0.0.1:7420@example.invalid`, whose real host is the part
    // after the `@`.
    let url = Url::parse(&raw).map_err(|err| format!("`{raw}` is not a URL: {err}"))?;
    if url.scheme() != "http" {
        return Err(format!("`{}` is not allowed; the daemon is loopback HTTP", url.scheme()));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("userinfo (`user@`) is not allowed in the daemon URL".to_string());
    }
    // Exact host match, for the same reason the daemon checks it that way:
    // a prefix test would accept `127.0.0.1.example.com`.
    let host = url.host_str().unwrap_or("");
    if !matches!(host, "127.0.0.1" | "localhost" | "[::1]") {
        return Err(format!("`{host}` is not a loopback host"));
    }
    Ok(url)
}

fn main() {
    let url = match daemon_url() {
        Ok(url) => url,
        Err(message) => {
            eprintln!("contexpect-desktop: refusing to start: {message}");
            std::process::exit(2);
        }
    };

    tauri::Builder::default()
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.clone()))
                .title("Contexpect")
                .inner_size(1440.0, 900.0)
                .min_inner_size(360.0, 480.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start the Contexpect desktop shell");
}

#[cfg(test)]
mod tests {
    use super::daemon_url;

    fn with_url<T>(value: &str, f: impl FnOnce() -> T) -> T {
        // Tests run single-threaded over this variable.
        unsafe { std::env::set_var("CONTEXPECT_DAEMON_URL", value) };
        let out = f();
        unsafe { std::env::remove_var("CONTEXPECT_DAEMON_URL") };
        out
    }

    #[test]
    fn only_a_plain_loopback_http_url_is_accepted() {
        for ok in ["http://127.0.0.1:7420", "http://localhost:7420/doctor", "http://[::1]:7420"] {
            assert!(with_url(ok, daemon_url).is_ok(), "{ok}");
        }
        for bad in [
            "https://127.0.0.1:7420",
            "http://127.0.0.1.evil.example:7420",
            "http://127.0.0.1:7420@example.invalid",
            "http://user:pw@127.0.0.1:7420",
            "http://evil.example",
            "ftp://127.0.0.1",
            "not a url",
        ] {
            assert!(with_url(bad, daemon_url).is_err(), "{bad}");
        }
    }
}
