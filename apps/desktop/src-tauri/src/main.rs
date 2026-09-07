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

use tauri::{WebviewUrl, WebviewWindowBuilder};

/// Where the daemon serves the UI.
///
/// Overridden with `CONTEXPECT_DAEMON_URL`. Only loopback origins are
/// accepted: a shell that would load an arbitrary URL is a browser, and this
/// window is not sandboxed like one.
fn daemon_url() -> Result<String, String> {
    let raw = std::env::var("CONTEXPECT_DAEMON_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:7420".to_string());
    let Some((scheme, authority)) = raw.split_once("://") else {
        return Err(format!("`{raw}` is not a URL"));
    };
    if scheme != "http" {
        return Err(format!("`{scheme}` is not allowed; the daemon is loopback HTTP"));
    }
    // Exact host match, for the same reason the daemon checks it that way:
    // a prefix test would accept `127.0.0.1.example.com`.
    let host = match authority.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    };
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err(format!("`{host}` is not a loopback host"));
    }
    Ok(raw)
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
            let parsed = url
                .parse()
                .map_err(|err| format!("cannot parse `{url}`: {err}"))?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(parsed))
                .title("Contexpect")
                .inner_size(1440.0, 900.0)
                .min_inner_size(360.0, 480.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start the Contexpect desktop shell");
}
