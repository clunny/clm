//! The issue form a person who isn't invited to the web app is sent to (ci/early-access-form).

use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml::Value;

fn github(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.github")
        .join(name)
}

fn read(name: &str) -> String {
    fs::read_to_string(github(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn early_access_form_asks_one_question_and_no_email() {
    // req:ci/early-access-form@83204c7f
    let text = read("ISSUE_TEMPLATE/early-access.yml");
    let form: Value = serde_yaml::from_str(&text).expect("early-access.yml parses");
    assert_eq!(form["name"], Value::String("Request early access".into()));
    let labels = form["labels"].as_sequence().expect("labels");
    assert!(labels.contains(&Value::String("early-access".into())));
    let description = form["description"].as_str().expect("description");
    assert!(description.contains("https://clm-app.dev/guide/quickstart"));
    assert!(description.contains("invite-only") && description.contains("early access"));
    let required = form["body"]
        .as_sequence()
        .expect("body")
        .iter()
        .filter(|field| field["validations"]["required"] == Value::Bool(true))
        .count();
    assert_eq!(required, 1, "exactly one required field");
    assert!(!text.to_lowercase().contains("email"), "asks for no email");
}
