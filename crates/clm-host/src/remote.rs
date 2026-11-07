//! What a remote's URL says about its host, and the steps to set a repository on it up for
//! CLM. Neither runs a process.

use clm_config::Host;

/// The host part of a remote URL in the forms `https://[user@]host[:port]/...`,
/// `http://...`, `ssh://[user@]host[:port]/...` and `user@host:path`, lower-cased.
fn host_part(url: &str) -> Option<String> {
    let url = url.trim();
    let authority = match url.split_once("://") {
        Some((scheme, rest))
            if ["https", "http", "ssh"]
                .iter()
                .any(|s| scheme.eq_ignore_ascii_case(s)) =>
        {
            rest.split(['/', '?', '#']).next()?
        }
        Some(_) => return None,
        None => url.split_once(':')?.0,
    };
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if url.contains("://") {
        host.split(':').next()?
    } else {
        host
    };
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// The `host` setting a remote's URL names (`host-for-remote`): `github`, `azure`, or `git`
/// for any other URL.
pub fn host_for_remote(url: &str) -> Host {
    match host_part(url).as_deref() {
        Some("github.com" | "ssh.github.com") => Host::Github,
        Some("dev.azure.com" | "ssh.dev.azure.com") => Host::Azure,
        Some(h) if h.ends_with(".visualstudio.com") => Host::Azure,
        _ => Host::Git,
    }
}

/// The lines a person follows to set a repository on `host` up for CLM
/// (`host-setup-steps`).
pub fn setup_steps(host: Host) -> Vec<String> {
    match host {
        Host::Azure => vec![
            "Install the Azure CLI with its `azure-devops` extension \
             (`az extension add --name azure-devops`) and sign in with `az login`."
                .into(),
            "If `main` has branch policies, grant the identity that runs `clm land` Azure \
             DevOps' `Bypass policies when pushing` permission on `main`."
                .into(),
        ],
        Host::Github => vec![
            "If `main` is protected, add the identity that runs `clm land` to the bypass \
             list of the rule or ruleset that protects it."
                .into(),
        ],
        Host::Git => vec![
            "If the server protects `main`, let the identity that runs `clm land` push it.".into(),
        ],
        Host::None => Vec::new(),
    }
}
