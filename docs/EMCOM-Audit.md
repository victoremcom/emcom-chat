# EMCOM Chat: initial security and reliability audit

Date: November 24, 2026. Baseline: upstream `6dd8548ef52ffe71c139c86b4c5594ab6d935f81`, version 1.6.1.

## Repository and scope

The connected GitHub identity is **victoremcom**, a personal account, with no organizations returned by the connector. The existing accessible repositories were emcom-ai, emcom-crm and emcom-ee-1500. None was modified. `victoremcom/emcom-chat` was absent and was created as a public GitHub fork of `ankurk91/google-chat-tauri`, preserving Git history, LICENSE.txt and upstream attribution.

The review branch is `emcom/security-notification-reliability`. `origin` is the EMCOM fork; `upstream` is the original repository. The user authorized committing and publishing this branch and opening a draft pull request on November 24, 2026. The main branch remains the upstream import until a separate merge decision. Branch/PR CI checks do not produce installers; no installer release or distribution is authorized.

This is an initial source/dependency audit and development prototype, not a penetration test or production security approval.

## Findings and changes

| Area | Finding | Change or status |
| --- | --- | --- |
| Remote native IPC | Five custom commands and event listen/unlisten are exposed to HTTPS mail.google.com and chat.google.com in the main window. No filesystem, shell, clipboard, autostart, reset or quit plugin commands are granted remotely. Gmail proper shares the mail origin. | Retain origin ACL; validate native window label, HTTPS/default port, lack of credentials, exact host and Chat path in **all five** custom commands. JS also excludes Gmail and `/chatty`. No new remote command was added. |
| Tauri transport | The protocol transport derives its ACL URL from the HTTP Origin header, which omits the document path. | Path-only capability narrowing would refuse legitimate calls on that transport. Enforce path from the native window URL instead. Event subscription remains origin-scoped. Linux has documented iframe-origin limitations; validate native behavior on both target OSes. |
| Auth hosts | The old short-label suffix heuristic accepts arbitrary names such as `accounts.google.abc.io`. | Replace with seven explicit Google account hosts already represented in upstream tests; test lookalikes and HTTP/credentials/nonstandard ports. Other country hosts require explicit review. This routes links; it does **not** prohibit all redirects or programmatic navigations. |
| Navigation/SSO | Top-level link handling is injected JS; `on_navigation` checks schemes only because Linux reports subframe navigation too. The five-minute “open every link” option allows arbitrary HTTPS sites in the window. | Retain this upstream behavior for compatibility and flag for review. EMCOM's actual identity provider and account restrictions are not yet known. A stricter top-frame navigation policy and approved SSO hosts remain rollout prerequisites. |
| Injected JS | Runs twice and must be idempotent; skips subframes, handles links, badges and Notification shims. It shares the remote page's JavaScript environment and is not isolated from Google code. | Keep idempotence and origin tests. Add regressions for Gmail scope, notification privacy and repeated liveness reports. Service-worker notifications originating when the page is suspended are not guaranteed by the shim. |
| Logging | URL/path redactors and line/length limits already exist. Notification “ID-like” strings can contain email, room, sender or message IDs even in debug. The rejected-link console fallback printed raw URLs. | Remove notification tag/data/key/value logging entirely; retain only local sequence/source. Redact fallback URLs. Native success logs say submitted, not displayed. Free-text `page_log` and third-party error strings remain a privacy risk: review logs before sharing and replace with structured telemetry before broad support use. |
| Dependencies | Baseline RustSec scan found cookie 0.18.2 and url 2.5.8 vulnerabilities, plus informational advisories including tokio I/O safety. | Apply compatible patched cookie 0.18.3, url 2.5.9 and tokio 1.53.3; retain lockfiles and validate again. No blanket dependency refresh or advisory suppression. |
| CI | Main-only Linux checks missed macOS/Windows compilation problems. Action tags were mutable; unsigned installers were built and uploaded by release automation. | Add Linux/macOS/Windows check matrix, locked Rust operations, immutable action SHAs and RustSec/npm audits. Replace release workflow with a manual job that fails with the release prerequisites. No installer build/upload/publish path remains in the working branch. Hosted CI has not run these changes. |
| Identity/update channel | Imported application identity and release links belonged to upstream. | Use EMCOM Chat / com.emcomsys.chat / emcom-chat, Windows per-user install, fork repository metadata and updates off by default. Preserve upstream package author, GPL and artwork notices. Linux purge paths are changed to EMCOM's identifier so a future fork package cannot delete the upstream app's data. Final EMCOM artwork is pending. |
| Close-to-tray | Existing close handler prevents closing, saves geometry, hides with a tray and minimizes without one. Explicit Quit exits. | Retained; real-device validation is required, including macOS app-hide and Windows tray restore behavior. |
| Autostart | Native plugin uses --hidden and a macOS LaunchAgent; opt-in, debug disabled, OS state queried. | Retained. Test from a signed installed build at actual login; startup must never strand the app without a tray. |
| Network/sleep | Startup TCP probe stopped permanently after success; connected flag meant “reported once.” | Keep observing TCP transitions after startup, add script liveness via the existing unread command every rescan, flag stale reports after two minutes, and add native diagnostics. TCP and script activity do not prove Google message delivery. |
| Recovery | Blind reloading can lose drafts or interrupt SSO. | Initial failed Chat loads may retry; authentication URLs and any page that has reported in are left alone. Live-session reconnection remains Google's responsibility. Manual File > Reload is documented after saving drafts. Timer behavior during OS sleep/webview throttling is unverified. |
| Notification diagnostics | Native test notification was debug-tray-only. Plugin errors were logged but successful submission had no evidence. | Add Help > Send Test Notification and Connection Diagnostics in all builds without granting remote access. OS submission is logged at debug. Test sends do not verify actual message delivery. |
| Notification clicks/sound | The current macOS/Windows integration has no click callback. The shim reports granted without querying OS authorization; existing sound behavior relies on Chat/system behavior. | **Unresolved.** Do not promise click-to-conversation, native permission health or reliable sound. These need platform implementation and installed-app tests. |

## Security model limits

Google authentication is loaded directly in the OS webview; this fork adds no credential proxy or server. The device holds login cookies and web storage in the app's separate profile. That is not an assurance that the wrapper is harmless: injected code runs beside the authenticated page, and a compromised native dependency runs with the user's OS permissions.

`withGlobalTauri` remains enabled because the activation listener uses the documented event API. Its presence does not bypass the ACL. `csp: null` is inherited; the local placeholder is not the Google application, and setting a local CSP does not harden Google's remotely served content. OS webview patching, least-privilege native commands and supply-chain controls remain necessary.

References used for the security model: [Tauri capabilities](https://v2.tauri.app/security/capabilities/), [Tauri permissions](https://v2.tauri.app/security/permissions/), [notification plugin](https://v2.tauri.app/plugin/notification/). The locked Tauri 2.12.2 `src/ipc/protocol.rs` and `src/webview/mod.rs` were also inspected to confirm remote command ACL enforcement and Origin-header behavior.

## Validation

Final checks on this Apple Silicon Mac:

- Rust 1.90: **75 unit tests passed**, zero failures; locked dependency build.
- `cargo clippy --locked ... -- -D warnings`: passed.
- `cargo fmt ... --check`: passed; `git diff --check`: passed.
- Injected script syntax and **97 JavaScript regression checks**: passed.
- Frozen pnpm 12.21.0 installation with package scripts disabled: passed; direct project `pnpm audit` reports zero advisories.
- RustSec cargo-audit 0.23.0 against the November 24 database (1,566 advisories; initial fetch commit `1d328b6bb9f2f350bf65c1c81ab68781cb9e018a`): **zero vulnerability entries** after patching cookie/url/tokio. There remain five unmaintained-package warnings (chrono, proc-macro-error, zbus_names, zvariant, zvariant_derive) and **RUSTSEC-2024-0429**, the Linux-only glib 0.18.5 unsound iterator warning. Upstream's GTK3 dependency constrains glib below the fixed 0.20 version; no advisory was suppressed. This is not a clean bill of health.
- JSON configuration parsing and Linux uninstall-script shell syntax: passed.
- GPL LICENSE.txt is unchanged from the upstream import.

The Apple Silicon debug executable is compiled without bundling/installing. No executable has been launched, signed, notarized or distributed. Windows/Intel compilation and hosted CI remain unverified. Real notification receipt, permission prompts, sound, click activation, autostart, SSO and recovery need human/device tests; unit tests and native submission logs do not establish those outcomes.

## Required acceptance tests before employee distribution

Run on macOS 15+ Intel and Apple Silicon, and Windows 10/11 x64, using company-signed installed builds and a test Workspace account. Record app/OS/webview versions, timestamps and results without message bodies or identities in logs.

1. Sign in, MFA/SSO, sign out and sign in again; verify no session is completed in the wrong browser. Test every regional/identity-provider host actually used by EMCOM. Confirm Google Workspace restrictions reject unauthorized accounts as intended.
2. Native IPC rejects Gmail proper, HTTP, lookalike hosts, unexpected windows and arbitrary remote pages. Chat's notification, badge and keyboard commands still work on both native IPC transports. Verify redirects/new windows do not escape the approved auth policy.
3. Test notification with OS permission granted and denied, Focus/Do Not Disturb on and off. Confirm identity, banner, Notification Center/Action Center history and sound. Denial must be visible and actionable.
4. Receive real direct and space messages while focused, unfocused, minimized and closed to tray. Check badge, sound, duplicates and missing notifications. Do not infer success from a debug synthetic activation.
5. Enable launch at login, restart/login, verify one hidden process and a usable tray. Launch twice, restore, close again, and Quit. Repeat with no tray available.
6. Disconnect Wi-Fi, change networks/VPN, launch offline, sleep for 5 minutes and overnight, then wake/reconnect. Verify real incoming messages and notifications resume. Check stale-report diagnostics and that recovery does not discard an unsent draft or interrupt MFA.
7. Implement and test macOS/Windows notification activation before claiming click-to-open. A generic app launch is not proof of routing to the correct conversation.
8. Run a multi-day pilot with a few designated testers only after signing/notarization and security review. Log missed-message counts and recovery times. Provide rollback/uninstall instructions that preserve unrelated app data.

## Release blockers

- macOS Developer ID signing/notarization and Windows company code signing are not configured.
- Windows, Intel macOS and real installed notification/permission/SSO/login/sleep tests have not run.
- No EMCOM identity-provider hostname/account policy is supplied; navigation and the broad temporary SSO option require review.
- Notification click integration, native permission reporting and sound reliability remain unimplemented/unproven on macOS/Windows.
- Residual dependency advisories and free-text logging need review as described in final scan results.
- This branch is for draft review only. Remote main retains upstream's dormant release workflow until the disabled-release change is merged. Do not tag or dispatch releases from remote main; the user has explicitly prohibited installer releases.


## CI follow-up: Windows path redaction

The first Windows CI run compiled and passed Clippy, then failed two path
redaction tests: `/media/jane-usb/report.pdf` and `/home/jane/notes.txt` were
logged unchanged. On Windows, a rooted path without a drive is not
`Path::is_absolute()`. Redaction now treats both roots and Windows prefixes
(including drive-relative paths) as locations that must be reduced to a file
name outside the known home directory. Native Windows regression coverage also
checks drive-qualified paths, drive-relative paths and UNC shares, with and
without a known home. Tests now pass the home explicitly rather than mutating
HOME/USERPROFILE in a multithreaded test process. No failing test was skipped
and no privacy assertion was weakened.
