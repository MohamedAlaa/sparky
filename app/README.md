# Sparky

A native Rust / egui mail workspace. On macOS it loads the installed Apple San Francisco font, retaining egui's fallback glyphs. Apple font files are not bundled or redistributed.

## Local mail engine

`sparky.sqlite3` replaces JSON persistence. On first launch, the existing `mailbox.json` is migrated without deleting the original. A successful migration is recorded so an empty mailbox stays empty. Failed migration is shown in the app and leaves the JSON intact.

SQLite3 uses WAL, a 256 MiB mmap ceiling, a five-second busy timeout and independent reader/writer handles. FTS5 indexes sender, subject and body with Unicode tokenization and diacritic removal; search combines prefix tokens with AND. Mail and its search index commit atomically. The system SQLite library must include FTS5 (as macOS does).

Conversations use account-scoped remote Gmail thread IDs or Graph conversation IDs first. The local graph connects Message-ID, References and In-Reply-To, including shared ancestors that have not been downloaded. Distinct authoritative remote conversations and different accounts are never merged. Subjects alone do not merge unrelated mail.

## Account adapters

Open **Connect email account** in the sidebar (also in **Settings** and the command center) to add Gmail, Outlook / Microsoft 365, or other TLS IMAP/SMTP accounts. For Gmail, click **Continue with Google**, choose your account in your browser, and approve inbox reading and sending. Your email address and profile name come from Google; there is no token to copy. Graph still uses an email address and OAuth access token; IMAP uses an app password and IMAP/SMTP hosts. **Connect & check inbox** verifies access and imports mail in the background; failed connections remain editable with an error. Repeat to connect multiple accounts. **All accounts** above the inbox filters mail to a single address, including downloaded mail after disconnecting or restarting. The filter combines with folders, categories and search, and folder unread counts follow it. Choose **All accounts** again to restore the unified view. New messages use the filtered connected account as their From address. **Sync all accounts** updates the unified inbox; compose and replies include a From account chooser. Disconnect removes the saved credential from macOS Keychain while retaining downloaded mail. If Keychain cannot be updated, the account remains connected and an error is shown.

Connected accounts are stored in macOS Keychain and restored after restarting, including Google refresh tokens, profile names and IMAP/SMTP hosts. Gmail renews its access token before the first request after restarting and as needed while open. Accounts connected in older builds need to be reconnected once because those builds never saved their credentials; Sync all opens Settings and explains this when only downloaded mail remains. Keychain errors are shown in the app. Gmail requests openid, profile, gmail.readonly and gmail.send scopes; reconnect Google to grant profile access if your name is missing. Graph requires delegated Mail.Read and Mail.Send permissions; optional User.Read enables the Microsoft profile name. Missing profile access and IMAP accounts fall back to the email address. Graph browser sign-in and renewal are not implemented. No live provider calls were made during development because no credentials were supplied. Environment variables remain supported for one initial account: SPARKY_PROVIDER (`gmail`, `graph`, `imap`), SPARKY_EMAIL, SPARKY_SECRET, and IMAP-only SPARKY_IMAP_HOST / SPARKY_SMTP_HOST. Use implicit TLS hosts such as `imap.example.com:993` and `smtp.example.com:465`. Tokens pass to curl through stdin; private outbound payloads are deleted after use.

### Google sign-in setup (app owner, once)

**Start with the step-by-step guide: [docs/google-setup.md](../docs/google-setup.md)** (Gmail API, consent screen, test users, Desktop client, troubleshooting). Technical details follow.

The app uses Google's desktop authorization-code flow with PKCE S256, a random state, and a temporary callback bound only to `127.0.0.1` on an OS-assigned port. It opens the system browser, handles the callback automatically, verifies granted scopes, and obtains the account address from Gmail. Cancel, denial, invalid callbacks, and a three-minute timeout are handled without blocking the interface. Access and refresh tokens are stored only in macOS Keychain and never enter the mailbox database, app bundle or command-line arguments. Disconnect removes the saved session; it does not revoke Google's consent grant (which users can remove in their Google account).

1. In Google Cloud, create a project, enable the Gmail API, and configure the OAuth consent screen with `https://www.googleapis.com/auth/gmail.readonly` and `https://www.googleapis.com/auth/gmail.send`.
2. Create an OAuth client of type **Desktop app**, then obtain its client ID and client secret. During testing, add colleague accounts as test users. Broader distribution requires Google's consent verification for these scopes; testing grants can expire.
3. Download the Desktop app client JSON. In Sparky, open **Connect email account → Gmail → Set up your own Google sign-in**, paste the downloaded JSON, and click **Save Google setup**. This validates the desktop client and saves only its client ID and client secret to `~/Library/Application Support/Sparky/google-oauth.json`. Run `Package for colleagues.command` to include that registration in the app bundle, so colleagues can just click **Continue with Google**. Account tokens and mailbox data are never packaged. Alternatively, set `SPARKY_GOOGLE_CLIENT_ID` and `SPARKY_GOOGLE_CLIENT_SECRET` when building. Runtime environment variables take precedence, followed by build-time variables, bundled configuration, and local saved setup. Desktop client credentials are public app configuration, not a substitute for PKCE; never use a web/server client secret here. No Google client credentials are supplied in this repository. An unconfigured app explains the setup and disables sign-in until configuration is available; configuration alone does not complete Google verification.
4. Open the build, choose Gmail, and click **Continue with Google**. Approve both permissions. After the validated Google callback, Sparky automatically restores and focuses its existing window while it finishes connecting; the browser tab can be closed. Once the token exchange and Gmail profile verification succeed, the account appears connected and is saved in Keychain immediately. The initial inbox downloads in the background; a failed download leaves the verified account connected so **Sync all accounts** can retry. Verify the correct address is connected, sync, and send a test message. Cancel and decline sign-in to check recovery; declining in Google should also return to Sparky with an error. Invalid or unrelated callbacks must not activate the window. Live Google sign-in has not been verified without a configured Google Cloud client and an authorized account.

See [Google's desktop OAuth documentation](https://developers.google.com/identity/protocols/oauth2/native-app) for the provider setup.

**⌘ Shift P** (or **⌘ K**) opens searchable commands. Use up/down arrows, Enter to run, and Escape to close. **Reply** / **R** opens the writer below the conversation. Compose and replies focus the writing area at its start. The shared native rich editor supports bold, italic, underline, headings, code, and list/quote insertion. Formatting persists with local drafts and Sent messages, renders in conversations, and is sent as HTML with a plain-text MIME alternative for Gmail/SMTP (Graph uses HTML). Closing a writer saves a draft; Discard closes without saving current edits.

Sync imports up to 50 recent inbox messages per click and deduplicates existing messages. Local folder moves, read flags, stars and snoozes remain local. Remote deletions, flag writeback, incremental cursors, attachments, and automatic sync are not implemented. IMAP currently reads basic RFC mail and base64 bodies; full MIME and international charset decoding remain future work. No live provider calls were made during development because no credentials were supplied.

With a configured account, **Send** sends through Gmail REST, Graph, or SMTP and saves to Sent after the server accepts the message. Without an account, **Save locally** only saves on this device. The adapter does not retry failed sends automatically. A timeout can occur after a server accepts mail; check server Sent before retrying. No automatic background sends occur.

## Keyboard workflow

Press **?** outside a text field, or **⌘ /** anywhere, for the searchable keyboard guide. **⌘ K** and **⌘ Shift P** open the command center. Windows/Linux use Ctrl in place of ⌘. All bindings, command labels, scopes, tooltips and guide entries share the catalog in `src/shortcuts.rs`.

**Command palette (⌘ K / ⌘ Shift P).** Fuzzy search in the style of VS Code: type the letters in order, for example `gti` for Go to Inbox. Matched letters are highlighted, and whole-word and word-start matches rank first. With an empty query, the palette lists your five most recently used commands (saved in the mailbox database across restarts) and then every command grouped by section. Use ↑/↓ to move, Page Up/Page Down to jump, Enter to run, and Esc or a click on the dimmed backdrop to close. Commands that can't run right now stay listed and show the reason, such as "Select a message first".

**Search (/).** Searches every folder at once through the full-text index, in the style of Algolia. Results are grouped by folder, and the matching words are highlighted in the subject and in an excerpt of the message. Enter opens the result in its folder. The last item, **Filter <folder> by "…"**, keeps the query as a filter on the current list instead; the header then shows the query with a clear button. Your last six searches are remembered across restarts and can be removed individually.

**Keyboard shortcuts guide (⌘ / or ?).** Search by name, or filter with the group chips. **Record keys** captures the next key press without running it and shows which commands use it. Alternative bindings appear side by side, for example `J` or `↓`.

**Swipe.** Swipe a conversation left with two fingers or drag it to reveal **Archive** and **Delete**. Pulling past the open width meets rubber-band resistance. Choosing an action, or pressing E, # or H, slides the row out and closes the gap before the move is saved. Quitting mid-animation still applies the move.

Use **J/K** or **↓/↑** to move through conversations; **Enter** opens the selected message or resumes its draft. **C** composes, **R** replies, **E** archives, **S** stars, **H** snoozes for 24 hours, **U** toggles unread, **#** or **Delete** moves to Trash, and **Shift I** restores to Inbox. **/** (or clicking **Search mail**) opens the search window; **Esc** closes it, or clears an active list filter. **1–4** switch All, People, Newsletters and Notifications. **⌘ Shift Y** syncs accounts; **⌘ ,** opens settings. Account connection is available through command search.

Press and release **G**, then within 1.5 seconds press **I** (Inbox), **S** (Starred), **H** (Snoozed), **T** (Sent), **D** (Drafts), **A** (Archive), or **X** (Trash). The footer shows the available second keys. Invalid second keys cancel the sequence; Escape cancels it immediately.

While writing, **⌘ Enter** sends (or saves locally without an account), **Esc** saves and closes the draft, and **⌘ Shift Backspace** discards. **⌘ B/I/U** apply bold/italic/underline, **⌘ Shift H** applies a heading and **⌘ Shift X** applies code. Use Tab/Shift Tab to move through native form controls. Letter shortcuts are suppressed in text fields, writers and dialogs, and repeated held keys cannot trigger mail actions or resend a message. Escape closes the topmost guide, command center or settings before saving a writer.

## Share with colleagues on Mac

Double-click **Package for colleagues.command** in this folder (the build Mac
needs Rust and Apple's command line developer tools). It builds a release app,
ad-hoc signs it and produces **dist/Sparky-macOS-arm64.zip** on Apple silicon
or **dist/Sparky-macOS-x86_64.zip** on Intel. Send that ZIP to colleagues with
the matching Mac processor. The ZIP contains **Sparky.app** directly, not a DMG.
They double-click the ZIP, open the extracted folder, and drag **Sparky.app**
onto the included **Applications** shortcut (or into Applications in Finder).
They then open Sparky from Applications; no installer or Rust is needed.
The ZIP includes **START HERE.txt** with setup and testing instructions.
It also includes **GOOGLE ACCESS FIX.txt**, with steps for the app owner to
approve testers and resolve Google's tester-only 403 access block.
Re-run the command after making changes.

The purple S workspace logo is also the app's Finder and Dock icon. The
packager includes `assets/Sparky.icns`, and the native window embeds
`assets/Sparky.png`. Each packaging run also saves standalone copies of the
logo to `dist/Sparky.png` and `dist/Sparky.icns`. To regenerate both from the existing mark, run
`swift assets/make-icon.swift /tmp/Sparky.iconset` from this folder.

Only the executable, app icon, bundle metadata and instructions are packaged. Release
builds store mail in `~/Library/Application Support/Sparky/`; development
builds retain the existing source-folder mailbox. No developer mailbox, tokens,
source code or Apple fonts are copied into the package. The test app is not
Developer ID signed or notarized; see `TESTING.txt` for macOS opening guidance.
Public distribution will require Developer ID signing and notarization.

## Build and verify locally

```
cargo build --offline --locked
cargo test --offline --locked
```

The storage test checks WAL, mmap, FTS5, index deletion, empty-mailbox persistence and a reader snapshot surviving a simultaneous writer commit. Threading and provider normalization have fixture tests; live protocol behavior requires an account.

## AI email classification

Open **Settings → AI model → How does our classifier work?** for a plain-language walkthrough, examples, privacy details and correction instructions.

Choose **Apple Foundation Model** (the default, on-device), **Ollama on this Mac**, or **My own server** (an OpenAI-compatible API, including MLX). Apple requires macOS 26 or later, supported hardware and Apple Intelligence enabled. Ollama requires a running local model. A custom server receives email text at the configured address and can be remote; use HTTPS for a remote service. Sparky never automatically falls back to another provider.

Use **Test model connection**, then optionally enable **Automatically label newly downloaded emails**. Newly imported messages are queued after sync; re-sync preserves existing labels. This does not add automatic polling. **Auto-classify inbox** processes existing inbox mail for the selected account across all categories, regardless of search. **Auto-classify** on a message handles one existing message.

Each request includes the sender, subject and a shortened body, with long text and quoted replies trimmed. Relevant user-corrected examples from the same account are included when the context budget permits, prioritizing matching senders and then shared subject words. Examples contain sender, subject, category and labels, never the old email body, and are sent only to the selected provider. The model first tries People (human messages), Newsletters (bulk mail/promotions), or Notifications (alerts/receipts), plus existing Work, Personal, Ideas, Social Media, Subscriptions, Banking and user-approved custom labels. Subscriptions covers recurring service receipts, membership charges, renewals, cancellations and account suspensions, including Spanish Amazon Prime receipts; it excludes newsletters and ordinary shopping receipts. Banking covers messages from your banks about your accounts, statements, transfers, deposits, balances, card transactions and security alerts; it excludes merchant receipts and bank promotions. Both are built-in labels available to existing installations and all model providers, with criteria shown in AI settings. Each structured response includes confidence and optional new label criteria. Emails from pinterest.com and pinterestmail.com (including subdomains) always receive Social Media and go into Notifications, including digests and promotions. This fixed rule runs before saved corrections and bypasses the model. Recognized Disney+, Netflix, Spotify, Hulu and Paramount+ sender domains with clear account or membership lifecycle subjects are routed directly to Notifications with Subscriptions, including suspended accounts. This rule does not use brand mentions in the body and always defers to saved manual corrections. Other messages use model predictions.

Sparky checks responses against existing categories and approved labels. Results with at least 75% confidence replace the category and labels locally. Lower confidence or uncertain results leave the email unchanged and open **Settings → AI model → Suggested criteria**. Edit the suggested label name and matching description, choose an existing category, then **Accept criteria & label email**, or dismiss the review. Acceptance adds the label to this email and saves its criteria for future model requests. Pending reviews and edits survive restarting. If a model omits suggestions, an editable Needs review criterion is offered. Custom labels appear in the message label menu and alongside built-in labels in the sidebar. Manual edits after classification prevent a stale review from being accepted. Invalid or truncated responses keep current labels and report an error; nothing is written back to the mail provider. Use the message's category and label menu to correct mistakes. Changing a category or adding or removing a label automatically saves a learning example in the local database and clears stale reviews for that email. Accepting a reviewed classification also saves an example. The latest correction replaces the previous example for that email, survives restarts, and is preserved when reclassifying it, without calling the model. Similar future emails use relevant examples to guide the model rather than treating a sender as an unconditional rule. Model weights are not retrained. AI settings show the number of corrected emails. Manual changes made while a request is running are respected.

One background request runs at a time. Server context is configurable from 1,024 to 8,192 tokens, with a conservative UTF-8 byte budget and space reserved for instructions and output. Cancellation keeps completed results and ignores current and pending work; the current request may take up to 90 seconds to finish. Connection or inference errors stop the batch and appear in Settings; retry after resolving them. Custom API keys are session-only. Proxies and redirects are disabled for model server requests.

Automated tests use fixtures and do not download weights. Live model quality requires the selected provider to be available. The opt-in mock API test (`cargo test --offline --locked local_api -- --ignored`) needs a loopback TCP listener and is skipped in restricted environments.
