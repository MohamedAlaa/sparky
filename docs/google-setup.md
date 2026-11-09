# Connect Gmail: Google Cloud setup

Sparky signs in to Gmail with **your own** Google OAuth client. This repo ships
no Google credentials, so do this once (about 10 minutes). After that,
**Continue with Google** works for you and for anyone you add as a tester.

> [!NOTE]
> Only Gmail needs this. IMAP/SMTP accounts just need an app password, and
> Outlook / Microsoft 365 uses a Graph access token.

**Contents**

1. [Create a project and enable the Gmail API](#1-create-a-project-and-enable-the-gmail-api)
2. [Configure the consent screen (Branding)](#2-configure-the-consent-screen-branding)
3. [Add yourself and your testers](#3-add-yourself-and-your-testers-audience) ⬅ the step everyone misses
4. [Add the Gmail scopes](#4-add-the-gmail-scopes-data-access)
5. [Create a Desktop app client](#5-create-a-desktop-app-oauth-client)
6. [Paste the JSON into Sparky](#6-paste-the-json-into-sparky)
7. [Sign in](#7-sign-in)
8. [Share a configured build](#8-share-a-configured-build-optional)
9. [Troubleshooting](#troubleshooting)

---

## 1. Create a project and enable the Gmail API

1. Open **[console.cloud.google.com](https://console.cloud.google.com/)** and sign in.
2. Open the project picker at the top and choose **New project**. Name it
   `Sparky` and click **Create**. Make sure it's the selected project.
3. Go to **☰ → APIs & Services → Library**, search for **Gmail API**, open
   it, and click **Enable**. If it says **Manage**, it's already on.

Use this same project for every step below.

## 2. Configure the consent screen (Branding)

1. Go to **☰ → Google Auth platform → Branding**. Older consoles call this
   **APIs & Services → OAuth consent screen**. Click **Get started** if
   you're asked to.
2. **App name:** `Sparky`. **User support email:** your address. Click **Next**.
3. **Audience:**
   - **External** for personal Gmail accounts or testers outside your
     organization. This is the usual choice.
   - **Internal** only if every user is in your Google Workspace organization.
     Internal apps have no tester list and need no verification.
4. Enter your contact email, accept the policy, then **Continue → Create**.

## 3. Add yourself and your testers (Audience)

An **External** app starts in **Testing**. While it's in Testing,
**only accounts on the Test users list can sign in.** Everyone else gets
*"Access blocked: Sparky has not completed the Google verification process"*
(error 403 `access_denied`).

1. Go to **Google Auth platform → Audience**.
2. Leave **Publishing status** as **Testing**.
3. Under **Test users**, click **+ Add users**.
4. Enter **your own Gmail address** and the Google sign-in address of each
   person who'll use your build, then click **Save**.
5. Check that every address now appears in the list.

Tips:

- Use the exact account people sign in with, not an alias or a mailing list.
- You can add up to 100 test users. Adding one takes effect right away and
  needs no new build.
- Testing grants can expire, usually after 7 days. Sparky then asks you to
  sign in again.

## 4. Add the Gmail scopes (Data Access)

1. Go to **Google Auth platform → Data Access → Add or remove scopes**.
2. Find and tick these two. Filtering by `gmail.` helps:

   ```
   https://www.googleapis.com/auth/gmail.readonly
   https://www.googleapis.com/auth/gmail.send
   ```

   If they aren't listed, paste both URLs into **Manually add scopes**, one per
   line, then click **Add to table**.
3. Click **Update**, then **Save**.

`gmail.readonly` appears under *restricted scopes* and `gmail.send` under
*sensitive scopes*. That's expected. Sparky also requests `openid` and
`profile` to show your name; those don't need to be added here.

## 5. Create a Desktop app OAuth client

1. Go to **Google Auth platform → Clients → + Create client**.
2. **Application type:** **Desktop app**. Don't pick Web application, and
   don't use an API key or service account.
3. **Name:** `Sparky Desktop`, then click **Create**.
4. Click **Download JSON**. You can also download it later from the client's
   details page. The file looks like this:

   ```json
   {"installed":{"client_id":"1234…apps.googleusercontent.com","client_secret":"GOCSPX-…", …}}
   ```

> [!WARNING]
> Keep this JSON out of git. Sparky's `.gitignore` already blocks
> `google-oauth.json`. For desktop apps Google treats the client secret as
> public app configuration, with PKCE protecting the sign-in, but there's
> still no reason to publish it.

## 6. Paste the JSON into Sparky

1. Run Sparky (`cd app && cargo run --release`).
2. Click **Connect email account → Gmail → Set up Google sign-in · app owner**.
3. Open the downloaded JSON in TextEdit, copy **all** of it, paste it in, and
   click **Save Google setup**.

Sparky checks that this is a Desktop client and saves only the client ID and
secret to:

```
~/Library/Application Support/Sparky/google-oauth.json
```

**Alternative, using environment variables:**

```bash
export SPARKY_GOOGLE_CLIENT_ID="1234…apps.googleusercontent.com"
export SPARKY_GOOGLE_CLIENT_SECRET="GOCSPX-…"
cargo run --release        # set them at build time to bake them into the binary
```

When several are present, Sparky uses the first one it finds, in this order:
runtime environment variables, build-time environment variables, a
`google-oauth.json` bundled in the app, then the saved setup above.

## 7. Sign in

1. **Connect email account → Gmail → Continue with Google.**
2. Your browser opens. Pick an account **that's on the Test users list**.
3. Google warns that it *"hasn't verified this app"*. That's expected for a
   Testing app. Click **Continue**.
4. Tick **both** permissions (read and send) and click **Continue**.
5. The browser tab says you can close it, and Sparky comes back to the front
   with the account connected. Your inbox downloads in the background.

Your refresh token is saved in the **macOS Keychain**, so you stay signed in
across restarts. It never goes into the mailbox database, the app bundle or
logs. Repeat for more Gmail accounts.

## 8. Share a configured build (optional)

So your testers don't need their own Cloud project:

1. Finish step 6 on your Mac.
2. Double-click `app/Package for colleagues.command`. It bundles your Desktop
   client config, but never your tokens or mail, into
   `app/dist/Sparky-macOS-<arch>.zip`.
3. Add each tester to **Test users** (step 3), then send them the ZIP.
   They just click **Continue with Google**.

---

## Troubleshooting

| What you see | Fix |
|---|---|
| **403 `access_denied` / "has not completed the Google verification process"** | The account isn't a test user. Add it in **Audience → Test users** (step 3), check it's exactly the address being chosen, and try again. No rebuild is needed. |
| **"Google sign-in is not configured for this copy of Sparky"** | Do step 6, or set the environment variables. |
| **"Choose a Desktop app client"** | You pasted a Web application client. Create a **Desktop app** client (step 5). |
| **Signed in but sync fails with missing permissions** | Confirm both scopes (step 4). In Sparky, disconnect the account and sign in again, ticking **both** boxes. |
| **Worked yesterday, now asks to sign in again** | Testing grants expire. Sign in again. |
| **Still 403 with the right address** | The Gmail API, consent screen and client must be in the **same project**: compare the `client_id` in your JSON with **Clients**. A Workspace admin may also block unverified apps. |

### Going beyond 100 testers

Publishing the app for anyone to use with `gmail.readonly` (a restricted
scope) requires Google's
[OAuth verification](https://support.google.com/cloud/answer/13463073), which
includes a security assessment. For personal use and small teams, Testing
mode with a tester list is enough.

**Google references:**
[Configure OAuth consent](https://developers.google.com/workspace/guides/configure-oauth-consent) ·
[Gmail API scopes](https://developers.google.com/workspace/gmail/api/auth/scopes) ·
[OAuth for desktop apps](https://developers.google.com/identity/protocols/oauth2/native-app)
