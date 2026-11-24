//! Is there a network at all, at startup?
//!
//! The window points straight at a remote page, so with no route to the
//! internet the app shows whatever the webview shows for a failed load -- which
//! on WebKitGTK is a bare error page with no indication that this is an app
//! that will work again later. Saying so once, in the desktop's own
//! notification, costs one TCP connection.
//!
//! It also has to put the window right afterwards, because the page the
//! webview is left showing cannot do it: WebKit will not re-navigate a
//! stand-in error document to the URL it is standing in for. Measured against
//! a local server started only after the load had already failed -- an `<a>`
//! pointing at that URL, `location.href = location.href` and
//! `location.reload()` all did nothing at all, while navigating to any *other*
//! URL worked immediately. So joining wifi after launching offline would leave
//! the app stuck on the error page for as long as it was open.
//!
//! Initial failed loads are retried after the network returns. EMCOM keeps
//! observing TCP reachability and page-script liveness thereafter, without
//! automatically reloading a live document or interrupting authentication.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::features::window::MAIN;
use crate::state::AppState;

/// Where to knock. Chat itself, so this answers the question that matters --
/// can we reach the thing the window is about -- rather than whether some
/// unrelated captive-portal endpoint is up.
const PROBE_HOST: &str = "chat.google.com:443";

/// How long each attempt may take. Short: a DNS lookup and a handshake-less
/// connect on a working network are well under a second, and a hung one should
/// not hold the retry schedule up.
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// Waits between attempts, in seconds. About a minute of waiting, front-loaded:
/// a machine that is already online answers the first attempt, and a laptop
/// joining wifi usually manages within the last of them. Measured end to end
/// against an address that never answers: 84s, because every failed attempt
/// also spends its connect timeout.
const BACKOFF_SECS: [u64; 5] = [2, 4, 8, 15, 30];

/// How often to look again once that schedule has run out.
///
/// Half a minute: slow enough to be nothing at all in the way of battery or
/// bandwidth, quick enough that the window is showing Chat within moments of
/// the wifi coming back rather than whenever the user next thinks to check.
const RECHECK: Duration = Duration::from_secs(30);

/// Where to knock instead, for testing. Debug builds only: an environment
/// variable that can silently make the app think it is offline has no business
/// in a release.
///
/// ```text
/// GOOGLE_CHAT_PROBE_HOST=192.0.2.1:443 ./google-chat-tauri   # TEST-NET-1, never routes
/// ```
#[cfg(debug_assertions)]
fn probe_host() -> String {
    std::env::var("GOOGLE_CHAT_PROBE_HOST").unwrap_or_else(|_| PROBE_HOST.to_string())
}

#[cfg(not(debug_assertions))]
fn probe_host() -> String {
    PROBE_HOST.to_string()
}

/// A TCP connection to the host the app is about.
///
/// This is only DNS plus TCP reachability, not TLS/HTTP or proof of Chat's
/// message transport. Captive portals and intercepting proxies can mislead it;
/// do not present this as an end-to-end delivery check.
pub fn is_online() -> bool {
    let host = probe_host();
    let addresses = match host.to_socket_addrs() {
        Ok(addresses) => addresses,
        // DNS failure is itself the answer.
        Err(e) => {
            log::debug!("connectivity: cannot resolve {host}: {e}");
            return false;
        }
    };

    for address in addresses {
        if TcpStream::connect_timeout(&address, PROBE_TIMEOUT).is_ok() {
            return true;
        }
    }

    false
}

/// Probe across the first minute, and tell the user once if nothing answers.
///
/// Returns immediately; the waiting happens on its own thread, because this is
/// called from `setup` and a minute of sleeping there would be a minute of no
/// window.
pub fn check_at_startup(app: &AppHandle) {
    let app = app.clone();

    std::thread::spawn(move || {
        let started = std::time::Instant::now();

        if is_online() {
            log::info!("connectivity: online");
            monitor(&app);
            return;
        }

        for (attempt, wait) in BACKOFF_SECS.iter().enumerate() {
            log::debug!(
                "connectivity: offline, retrying in {wait}s ({}/{})",
                attempt + 1,
                BACKOFF_SECS.len()
            );
            std::thread::sleep(Duration::from_secs(*wait));

            if is_online() {
                log::info!(
                    "connectivity: online after {} attempt(s)",
                    attempt + 2 // the first attempt was before the loop
                );
                // The first probe failed, so the window's own load almost
                // certainly failed with it. Put Chat back.
                load_chat(&app);
                monitor(&app);
                return;
            }
        }

        // Elapsed, not the sum of the waits: each failed attempt also spends
        // up to PROBE_TIMEOUT before it gives up, so the real span is longer
        // than the schedule suggests.
        log::warn!(
            "connectivity: no network after {}s; telling the user",
            started.elapsed().as_secs()
        );
        crate::features::notifications::show(
            &app,
            0,
            "No internet connection",
            Some("Google Chat could not be reached. It will load once you are back online."),
        );

        // And keep looking, because that notification promises it. Nothing else
        // is watching: the error page in the window has no way to retry itself,
        // and there is no `did-fail-load` equivalent in wry for Rust to hang a
        // retry on. Ends on the first success.
        loop {
            std::thread::sleep(RECHECK);

            if is_online() {
                log::info!(
                    "connectivity: network back after {}s; loading Chat",
                    started.elapsed().as_secs()
                );
                load_chat(&app);
                monitor(&app);
                return;
            }
        }
    });
}

/// Keep observing after startup. Do not reload a live page automatically:
/// that would discard drafts and can interrupt authentication. Google owns its
/// reconnect logic; this monitor detects transport loss and missing JS reports.
fn monitor(app: &AppHandle) {
    let mut was_online = true;
    let mut was_stale = false;
    loop {
        std::thread::sleep(RECHECK);
        if app.state::<AppState>().is_quitting() {
            return;
        }
        let online = is_online();
        if online != was_online {
            log::info!("connectivity: TCP reachable={online}");
            if online {
                // Only an initial failed load may be replaced, never a page
                // which has reported in or an authentication page.
                load_chat(app);
            }
            was_online = online;
        }
        let stale = app
            .state::<AppState>()
            .page_report_age()
            .is_some_and(|age| age > Duration::from_secs(120));
        if stale != was_stale {
            log::warn!(
                "connectivity: page script report stale={stale}; this does not measure message delivery"
            );
            was_stale = stale;
        }
    }
}

/// Point the main window at Chat again.
///
/// Called from the probe thread, which is where it has to happen anyway:
/// `navigate` runs inline when it is already on the main thread, and off it the
/// request goes through the event loop instead.
fn load_chat(app: &AppHandle) {
    // The user may have got there first with the offline page's Try again
    // button. If `chat.js` is reporting in then the window is already on Chat,
    // and navigating again would only throw away whatever they were doing.
    if app.state::<AppState>().is_connected() {
        log::debug!("connectivity: the page is already up; leaving it alone");
        return;
    }

    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    // Avoid navigating away from a sign-in or SSO flow that is in progress.
    if !window
        .url()
        .is_ok_and(|url| crate::urls::is_chat_page(&url))
    {
        log::debug!("connectivity: not on a Chat URL; leaving navigation alone");
        return;
    }
    match crate::urls::APP_URL.parse() {
        Ok(url) => {
            let _ = window.navigate(url);
        }
        Err(e) => log::error!("connectivity: cannot parse APP_URL: {e}"),
    }
}

/// The schedule's own total, for the test that keeps it honest.
#[cfg(test)]
fn total_wait() -> u64 {
    BACKOFF_SECS.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_retry_schedule_covers_about_a_minute() {
        // Long enough for wifi to associate after a login, short enough that
        // the answer still means something when it arrives.
        let total = total_wait();
        assert!((45..=90).contains(&total), "retries span {total}s");
    }

    #[test]
    fn backoff_never_shrinks() {
        assert!(
            BACKOFF_SECS.windows(2).all(|pair| pair[0] <= pair[1]),
            "waits must not get shorter: {BACKOFF_SECS:?}"
        );
    }

    #[test]
    fn the_probe_target_carries_a_port() {
        // to_socket_addrs on a bare host silently fails; the port is not
        // optional and is easy to lose in an edit.
        assert!(PROBE_HOST.contains(':'), "{PROBE_HOST} has no port");
    }
}
