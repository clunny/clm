//! `host_for_remote` and `setup_steps` (host/host-for-remote, host/host-setup-steps).

use clm_config::Host;
use clm_host::{host_for_remote, setup_steps};

#[test]
fn hosts_named() {
    // req:host/host-for-remote@25bf2426
    for (url, want) in [
        ("https://github.com/o/r.git", Host::Github),
        ("git@github.com:o/r.git", Host::Github),
        ("https://o@dev.azure.com/o/p/_git/r", Host::Azure),
        ("git@ssh.dev.azure.com:v3/o/p/r", Host::Azure),
        ("https://o.visualstudio.com/p/_git/r", Host::Azure),
        ("ssh://git@example.com/r.git", Host::Git),
        ("ssh://git@ssh.github.com:443/o/r.git", Host::Github),
        ("HTTPS://GitHub.com/o/r", Host::Github),
        ("https://GitHub.com:8443/o/r", Host::Github),
        ("", Host::Git),
    ] {
        assert_eq!(host_for_remote(url), want, "{url}");
    }
}

#[test]
fn a_name_in_the_path() {
    // req:host/host-for-remote@25bf2426
    for url in [
        "https://example.com/dev.azure.com/r.git",
        "https://o.visualstudio.com.example.org/r",
        "https://example.com/github.com/r.git",
        "https://notgithub.com/o/r",
        "git@example.com:github.com/r.git",
    ] {
        assert_eq!(host_for_remote(url), Host::Git, "{url}");
    }
}

#[test]
fn azure_steps() {
    // req:host/host-setup-steps@766cfd96
    let steps = setup_steps(Host::Azure).join("\n");
    for word in [
        "azure-devops",
        "az login",
        "Bypass policies when pushing",
        "clm land",
    ] {
        assert!(steps.contains(word), "{word}: {steps}");
    }
}

#[test]
fn github_git_and_none_steps() {
    // req:host/host-setup-steps@766cfd96
    let github = setup_steps(Host::Github).join("\n");
    assert!(github.contains("clm land"), "{github}");
    for word in ["az login", "azure-devops", "Bypass policies when pushing"] {
        assert!(!github.contains(word), "{word}: {github}");
    }
    let git = setup_steps(Host::Git).join("\n");
    assert!(git.contains("clm land"), "{git}");
    assert!(setup_steps(Host::None).is_empty());
}
