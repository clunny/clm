//! The version string `clm --version` prints. `build.rs` includes this file with `#[path]`;
//! `main.rs` includes it only for its test, since it takes the string the build produced.

/// `package`, followed by ` (<commit>)` when the build knows the commit it was built from.
pub fn version_string(package: &str, commit: Option<&str>) -> String {
    match commit {
        Some(commit) => format!("{package} ({commit})"),
        None => package.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::version_string;

    #[test]
    fn with_and_without_a_commit() {
        // req:clm/build-commit@b9775322
        assert_eq!(version_string("0.7.0", None), "0.7.0");
        assert_eq!(
            version_string("0.7.0", Some("0123456789ab")),
            "0.7.0 (0123456789ab)"
        );
    }
}
