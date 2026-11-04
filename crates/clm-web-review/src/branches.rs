//! `GET /api/repos/{o}/{r}/branches`: the spec branches the app's spec-branch views start
//! from (`web.worker/spec-branches`). Each `spec/*` ref whose review can open, by name, with
//! its head and its page: the open pull request on the default branch whose head is the
//! branch in this repository, not a fork's. Kept with the live state for 60 seconds.

use serde::{Deserialize, Serialize};

use crate::review;
use clm_web_platform::access::Access;
use clm_web_platform::platform::Platform;
use clm_web_platform::{gh, live};

/// A spec branch's page on the host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub number: u64,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecBranch {
    pub branch: String,
    pub head_sha: String,
    pub page: Option<Page>,
}

pub async fn list<P: Platform>(p: &P, access: &Access) -> Option<Vec<SpecBranch>> {
    live::cached(p, access, "branches", async || {
        let path = format!(
            "/repos/{}/git/matching-refs/heads/spec/",
            access.full_name()
        );
        let (refs, pulls) =
            futures::join!(gh::get_ok(p, access.auth(), &path), live::pulls(p, access));
        let (refs, pulls) = (refs?, pulls?);
        let page = |branch: &str| {
            pulls
                .iter()
                .find(|pr| pr.head == branch && pr.base == access.default_branch() && !pr.fork)
                .map(|pr| Page {
                    number: pr.number,
                    url: pr.url.clone(),
                    title: pr.title.clone(),
                })
        };
        let mut branches: Vec<SpecBranch> = refs
            .as_array()?
            .iter()
            .filter_map(|r| {
                let branch = r["ref"].as_str()?.strip_prefix("refs/heads/")?;
                let head_sha = r["object"]["sha"].as_str()?;
                review::is_spec_branch(branch).then(|| SpecBranch {
                    branch: branch.into(),
                    head_sha: head_sha.into(),
                    page: page(branch),
                })
            })
            .collect();
        branches.sort_by(|a, b| a.branch.cmp(&b.branch));
        Some(branches)
    })
    .await
}
