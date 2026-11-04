//! `web/wrangler.toml` deploys the Worker the setup made, and `web/README.md` records that
//! setup, as the web Design's Deploy, GitHub App and Setup paragraphs declare them.

use std::fs;
use std::path::Path;

use toml::Value;

/// A file by its path from the repository root.
fn repo_file(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("can't read {}: {e}", full.display()))
}

fn wrangler() -> Value {
    toml::from_str(&repo_file("web/wrangler.toml")).expect("web/wrangler.toml parses")
}

#[test]
fn deploys_the_clm_worker_on_its_custom_domain() {
    let w = wrangler();
    assert_eq!(w["name"].as_str(), Some("clm"));
    let routes = w["routes"].as_array().expect("routes");
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0]["pattern"].as_str(), Some("clm-app.dev"));
    assert_eq!(routes[0]["custom_domain"].as_bool(), Some(true));
}

#[test]
fn binds_the_kv_namespace() {
    let w = wrangler();
    let kv = w["kv_namespaces"].as_array().expect("kv_namespaces");
    assert_eq!(kv.len(), 1);
    assert_eq!(kv[0]["binding"].as_str(), Some("CLM"));
    assert_eq!(
        kv[0]["id"].as_str(),
        Some("9fa493aaa0a54d219b2dfebb5f681b9e")
    );
}

#[test]
fn sends_only_the_api_and_its_paths_to_the_worker() {
    // req:web/invited-users@cfd18219
    // req:web/site-served-directly@f8994877
    // `/_clm/*` reaches the Worker, which answers 404, so the invited list is never
    // served to someone without a session.
    let w = wrangler();
    let assets = &w["assets"];
    assert_eq!(assets["directory"].as_str(), Some("./dist"));
    let first: Vec<&str> = assets["run_worker_first"]
        .as_array()
        .expect("run_worker_first")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    // Only the API, sign-in, webhook, invited list and `/app`; the assets answer the rest.
    assert_eq!(first, ["/api/*", "/auth/*", "/webhook", "/_clm/*", "/app"]);
}

#[test]
fn the_worker_reads_the_build_time_assets_through_its_binding() {
    // req:web/invited-users@cfd18219
    // `_clm/` holds the invited list. The Worker reads it through the binding, and the
    // run_worker_first entry above keeps it from being served directly.
    let w = wrangler();
    assert_eq!(w["assets"]["binding"].as_str(), Some("ASSETS"));
    assert!(
        repo_file("crates/clm-web/src/worker.rs")
            .contains("const ASSETS_BINDING: &str = \"ASSETS\";")
    );
}

#[test]
fn readme_records_how_to_redo_the_setup() {
    let readme = repo_file("web/README.md");
    let kv_id = wrangler()["kv_namespaces"][0]["id"]
        .as_str()
        .expect("kv id")
        .to_string();
    for fact in [
        "GITHUB_APP_ID",
        "GITHUB_APP_PRIVATE_KEY",
        "GITHUB_CLIENT_ID",
        "GITHUB_CLIENT_SECRET",
        "GITHUB_WEBHOOK_SECRET",
        "COOKIE_KEY",
        "clm-spec-review/installations/new",
        "App ID 5433936",
        "## Rotate a credential",
        "openssl pkcs8 -topk8 -nocrypt",
        "openssl rand -base64 32",
        "wrangler kv namespace create clm-cache",
        "wrangler secret put",
        "gh secret set CLOUDFLARE_API_TOKEN",
        "gh secret set CLOUDFLARE_ACCOUNT_ID",
        &kv_id,
    ] {
        assert!(readme.contains(fact), "web/README.md names {fact}");
    }
}

#[test]
fn the_code_reads_the_designs_secrets_and_links_the_app() {
    let mut names = std::collections::BTreeSet::new();
    for krate in ["clm-web", "clm-web-platform"] {
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(krate)
            .join("src");
        for entry in fs::read_dir(&src).expect("src") {
            let text = fs::read_to_string(entry.expect("entry").path()).expect("source");
            for part in text.split("secret(\"").skip(1) {
                names.insert(part[..part.find('"').expect("name ends")].to_string());
            }
        }
    }
    assert_eq!(
        names.into_iter().collect::<Vec<_>>(),
        [
            "COOKIE_KEY",
            "GITHUB_APP_ID",
            "GITHUB_APP_PRIVATE_KEY",
            "GITHUB_CLIENT_ID",
            "GITHUB_CLIENT_SECRET",
            "GITHUB_WEBHOOK_SECRET",
            "VAPID_PUBLIC_KEY",
        ]
    );
    assert!(repo_file("crates/clm-web/src/worker.rs").contains("&\"pkcs8\".into()"));
    assert!(
        repo_file("web/src/main.ts")
            .contains("'https://github.com/apps/clm-spec-review/installations/new'")
    );
}

#[test]
fn no_secret_is_committed() {
    let w = wrangler();
    assert!(
        w.get("vars").is_none() && w.get("env").is_none(),
        "web/wrangler.toml has no [vars] or [env.*]: secrets are Worker secrets"
    );
    let readme = repo_file("web/README.md");
    assert!(
        !readme.contains("PRIVATE KEY-----"),
        "web/README.md holds no private key"
    );
}

#[test]
fn builds_with_worker_build_matching_the_pinned_worker_crate() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("Cargo.toml");
    let pin = manifest
        .lines()
        .find_map(|l| l.strip_prefix("worker = \"="))
        .and_then(|v| v.strip_suffix('"'))
        .expect("worker is pinned with =");
    let package =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/package.json"))
            .expect("web/package.json");
    let json: serde_json::Value = serde_json::from_str(&package).expect("package.json parses");
    let script = json["scripts"]["build:worker"]
        .as_str()
        .expect("build:worker");
    assert!(
        script.contains(&format!("worker-build --version {pin}")),
        "build:worker installs worker-build {pin}: {script}"
    );
    let build = wrangler()["build"]["command"]
        .as_str()
        .expect("build command")
        .to_string();
    assert!(build.contains("npm run build") && build.contains("npm run build:worker"));
}
