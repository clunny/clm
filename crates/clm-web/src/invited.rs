//! Who may sign in: the logins in the web Contract's invited-users list, which the build
//! writes to `/_clm/invited.json` (web/invited-users).

use clm_web_platform::platform::Assets;

const LIST: &str = "/_clm/invited.json";

/// Whether `login` is invited. GitHub logins are case-insensitive. Without a readable
/// list, nobody is.
pub(crate) async fn is_invited<P: Assets>(p: &P, login: &str) -> bool {
    let Some(text) = p.asset(LIST).await else {
        return false;
    };
    let logins: Vec<String> = serde_json::from_str(&text).unwrap_or_default();
    !login.is_empty() && logins.iter().any(|l| l.eq_ignore_ascii_case(login))
}

/// The page for a sign-in GitHub didn't say whose it was.
pub(crate) const NO_LOGIN: &str = "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>CLM</title><h1>Sign-in failed</h1><p>GitHub didn't say who you are. <a href=\"/auth/login\">Sign in again</a>.</p>";

/// Where someone not invited asks for early access: the one place the Worker names it.
const EARLY_ACCESS_URL: &str = "https://github.com/clunny/clm/issues/new?template=early-access.yml";

/// The page someone who isn't invited gets instead of a session.
pub(crate) fn not_invited_page() -> String {
    format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>CLM</title><h1>Not invited</h1><p>You're signed in to GitHub, but you're not invited to the CLM web app: it is in early access.</p><p>CLM works fully without it: follow the <a href=\"/guide/quickstart\">quickstart</a>. The web app is an optional extra for convenience.</p><p><a href=\"{EARLY_ACCESS_URL}\">Request early access</a> or go to the <a href=\"/\">homepage</a>.</p>"
    )
}
