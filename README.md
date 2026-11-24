# EMCOM Chat — development prototype

A company-maintained fork of [ankurk91/google-chat-tauri](https://github.com/ankurk91/google-chat-tauri), preserving **GPL-3.0-only** and upstream attribution. See [UPSTREAM.md](UPSTREAM.md) and [LICENSE.txt](LICENSE.txt).

**Not approved for employee distribution.** No EMCOM installers have been released. Release automation is disabled until macOS signing/notarization, Windows signing and real-device acceptance testing are in place. Do not bypass Gatekeeper or SmartScreen to deploy this prototype.

The app loads Google's own Chat website in the system webview. It adds a tray icon, unread badges, close-to-tray behavior and native notifications. It stores a separate webview login session on the device; no EMCOM server is required. It does not enforce an EMCOM-only account policy: Google Workspace administrators must enforce access policy.

## Current scope

- macOS 15+ on Apple Silicon and Intel; Windows 10/11 x64 remain the targets.
- Closing the window keeps the process running. Use Quit to exit.
- Preferences > Launch at Login is optional and unavailable in debug builds.
- Help > Send Test Notification tests native submission; it does not prove real message delivery.
- Help > Connection Diagnostics reports TCP reachability and age of the page script's last report. Neither verifies Google's message transport.
- A continuous monitor records network transitions and stale script activity, including after sleep. It does not reload a live document or discard drafts automatically. Reopen Chat, save your draft, and use File > Reload if needed.
- Notifications depend on Google Chat settings and the OS. Verify both banners and sounds with a real incoming message while hidden, after sleep and after reconnecting.
- Clicking a notification is **not implemented by the current macOS/Windows integration**. The shim's permission="granted" is not evidence of OS permission.
- Automatic GitHub release checks default to off. Manual checks and issue links point to this fork, never upstream installers.

## Development

See [docs/Development.md](docs/Development.md) for prerequisites and upstream architecture, and [docs/EMCOM-Audit.md](docs/EMCOM-Audit.md) for findings, validation and rollout blockers. The Rust compiler and dependency locks are retained from upstream.

```sh
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
node --check src-tauri/src/inject/chat.js
node scripts/chatjs-test.js
pnpm install --frozen-lockfile
pnpm tauri build --debug --no-bundle
```

CI checks Linux, macOS and Windows without producing or uploading installers. A local macOS build validates only the host architecture. Intel, Windows, Google Workspace authentication and actual notification behavior need separate validation.

## Authentication and privacy

Native IPC is limited to the main window's HTTPS Chat pages. Reviewed auth hosts are explicit: accounts.google.com, accounts.google.co.in, accounts.google.co.uk, accounts.google.com.au, accounts.google.de, accounts.google.fr and accounts.youtube.com. Other regional hosts require review before being added.

The upstream five-minute “Open Every Link in This Window” escape hatch remains present for external SSO. It is not an organization hostname allowlist and must not be treated as a security guarantee. A reviewed EMCOM identity-provider policy remains a rollout prerequisite.

Logs stay on-device; notification content and payload identifiers are not intentionally logged by the injected script. Review logs before sharing: remote pages can still call the free-text page_log command, and third-party library errors may contain data. Do not attach company logs to public GitHub issues without reviewing them.
