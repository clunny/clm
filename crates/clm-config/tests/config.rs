use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clm_config::{
    Affected, Approval, Defects, Detector, Health, Host, Land, Layer, NodeSize, Settings, load,
};
use tempfile::TempDir;

/// A temp dir holding `repo/` (with `clm.toml` when given), `xdg/clm/config.toml` (when given)
/// and `repo/clm.local.toml` (when added with `local`).
struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new(user: Option<&str>, repo: Option<&str>) -> Self {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("repo")).unwrap();
        if let Some(user) = user {
            fs::create_dir_all(dir.path().join("xdg/clm")).unwrap();
            fs::write(dir.path().join("xdg/clm/config.toml"), user).unwrap();
        }
        if let Some(repo) = repo {
            fs::write(dir.path().join("repo/clm.toml"), repo).unwrap();
        }
        Fixture { dir }
    }

    fn root(&self) -> PathBuf {
        self.dir.path().join("repo")
    }

    fn repo_file(&self) -> PathBuf {
        self.root().join("clm.toml")
    }

    fn user_file(&self) -> PathBuf {
        self.dir.path().join("xdg/clm/config.toml")
    }

    fn local_file(&self) -> PathBuf {
        self.root().join("clm.local.toml")
    }

    /// Adds `repo/clm.local.toml`: the repository has no `.git`, so it is read beside `clm.toml`.
    fn local(self, text: &str) -> Self {
        fs::write(self.local_file(), text).unwrap();
        self
    }

    /// Loads with `XDG_CONFIG_HOME` pointing at the fixture plus `vars`.
    fn load(&self, vars: &[(&str, &str)]) -> Result<Settings, clm_config::Error> {
        let mut env: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        env.insert(
            "XDG_CONFIG_HOME".into(),
            self.dir.path().join("xdg").display().to_string(),
        );
        load_with(Some(&self.root()), &env)
    }
}

fn load_with(
    root: Option<&Path>,
    env: &HashMap<String, String>,
) -> Result<Settings, clm_config::Error> {
    load(root, &|k: &str| env.get(k).cloned())
}

fn files() -> Fixture {
    Fixture::new(
        Some("defects = \"off\"\nautosync = false\n"),
        Some("defects = \"backlog\"\n"),
    )
}

#[test]
fn repository_over_user() {
    // req:config/layers@cf14f119
    let f = files();
    let s = f.load(&[]).unwrap();
    assert_eq!(s.defects.value, Defects::Backlog);
    assert_eq!(s.defects.layer, Layer::File(f.repo_file()));
    assert!(!s.autosync.value);
    assert_eq!(s.autosync.layer, Layer::File(f.user_file()));
}

#[test]
fn variable_over_files() {
    // req:config/layers@cf14f119
    let f = files();
    let s = f
        .load(&[("CLM_DEFECTS", "ask"), ("CLM_AUTOSYNC", "0")])
        .unwrap();
    assert_eq!(s.defects.value, Defects::Ask);
    assert_eq!(s.defects.layer, Layer::Env("CLM_DEFECTS"));
    assert!(!s.autosync.value);
    assert_eq!(s.autosync.layer, Layer::Env("CLM_AUTOSYNC"));
}

#[test]
fn autosync_variable_accepts_true_and_false_spellings() {
    // req:config/layers@cf14f119
    let f = Fixture::new(Some("autosync = false\n"), None);
    for (v, want) in [("0", false), ("false", false), ("1", true), ("true", true)] {
        let s = f.load(&[("CLM_AUTOSYNC", v)]).unwrap();
        assert_eq!(s.autosync.value, want, "CLM_AUTOSYNC={v}");
    }
}

#[test]
fn no_files_gives_defaults() {
    // req:config/layers@cf14f119
    let f = Fixture::new(None, None);
    let s = f.load(&[]).unwrap();
    assert_eq!(s.defects.value, Defects::Ask);
    assert_eq!(s.defects.layer, Layer::Default);
    assert!(s.autosync.value);
    assert_eq!(s.autosync.layer, Layer::Default);
}

#[test]
fn no_root_reads_only_the_user_file() {
    // req:config/layers@cf14f119
    let f = files();
    let env = HashMap::from([(
        "XDG_CONFIG_HOME".to_string(),
        f.dir.path().join("xdg").display().to_string(),
    )]);
    let s = load_with(None, &env).unwrap();
    assert_eq!(s.defects.value, Defects::Off);
    assert_eq!(s.defects.layer, Layer::File(f.user_file()));
}

#[test]
fn home_fallback_when_xdg_unset_or_empty() {
    // req:config/layers@cf14f119
    let dir = TempDir::new().unwrap();
    let file = dir.path().join(".config/clm/config.toml");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, "defects = \"off\"\n").unwrap();
    let home = dir.path().display().to_string();
    for xdg in [None, Some("")] {
        let mut env = HashMap::from([("HOME".to_string(), home.clone())]);
        if let Some(xdg) = xdg {
            env.insert("XDG_CONFIG_HOME".into(), xdg.into());
        }
        let s = load_with(None, &env).unwrap();
        assert_eq!(s.defects.value, Defects::Off, "XDG_CONFIG_HOME={xdg:?}");
        assert_eq!(s.defects.layer, Layer::File(file.clone()));
    }
}

#[test]
fn no_home_no_user_file() {
    // req:config/layers@cf14f119
    let s = load_with(None, &HashMap::new()).unwrap();
    assert_eq!(s.defects.layer, Layer::Default);
    assert_eq!(s.autosync.layer, Layer::Default);
}

fn error(f: &Fixture, vars: &[(&str, &str)]) -> String {
    f.load(vars).unwrap_err().to_string()
}

#[test]
fn unknown_key_names_file_and_key() {
    // req:config/invalid-settings@ec741034
    let f = Fixture::new(None, Some("defect = \"off\"\n"));
    let e = error(&f, &[]);
    assert!(e.contains("clm.toml"), "{e}");
    assert!(e.contains("`defect`"), "{e}");
}

#[test]
fn bad_defects_variable() {
    // req:config/invalid-settings@ec741034
    let e = error(&Fixture::new(None, None), &[("CLM_DEFECTS", "always")]);
    assert!(e.contains("CLM_DEFECTS"), "{e}");
    assert!(e.contains("always"), "{e}");
    assert!(e.contains("`defects`"), "{e}");
}

#[test]
fn bad_autosync_variable() {
    // req:config/invalid-settings@ec741034
    let e = error(&Fixture::new(None, None), &[("CLM_AUTOSYNC", "yes")]);
    assert!(e.contains("CLM_AUTOSYNC"), "{e}");
    assert!(e.contains("`autosync`"), "{e}");
}

#[test]
fn empty_variable_is_set_and_invalid() {
    // req:config/invalid-settings@ec741034
    let f = Fixture::new(None, None);
    assert!(error(&f, &[("CLM_AUTOSYNC", "")]).contains("CLM_AUTOSYNC"));
    assert!(error(&f, &[("CLM_DEFECTS", "")]).contains("CLM_DEFECTS"));
}

#[test]
fn unparseable_file() {
    // req:config/invalid-settings@ec741034
    let e = error(&Fixture::new(None, Some("defects = \n")), &[]);
    assert!(e.contains("clm.toml"), "{e}");
}

#[test]
fn bad_value_in_user_file() {
    // req:config/invalid-settings@ec741034
    let e = error(&Fixture::new(Some("defects = \"always\"\n"), None), &[]);
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("defects"), "{e}");
}

#[test]
fn bad_type_in_repository_file() {
    // req:config/invalid-settings@ec741034
    let e = error(&Fixture::new(None, Some("autosync = \"no\"\n")), &[]);
    assert!(e.contains("clm.toml"), "{e}");
    assert!(e.contains("autosync"), "{e}");
}

#[test]
fn get_gives_value_and_layer() {
    // req:config/config-get@e2bb5643
    let f = files();
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("defects").unwrap();
    assert_eq!(value, "backlog");
    assert_eq!(*layer, Layer::File(f.repo_file()));
    let (value, layer) = s.get("autosync").unwrap();
    assert_eq!(value, "false");
    assert_eq!(*layer, Layer::File(f.user_file()));
}

#[test]
fn get_unknown_key() {
    // req:config/config-get@e2bb5643
    let s = Fixture::new(None, None).load(&[]).unwrap();
    let e = s.get("colour").unwrap_err().to_string();
    assert!(e.contains("colour"), "{e}");
}

#[test]
fn error_keeps_its_source_apart_from_its_message() {
    // req:config/invalid-settings@ec741034
    let f = Fixture::new(None, Some("defect = \"off\"\n"));
    let e = f.load(&[]).unwrap_err();
    assert_eq!(e.source, Some(f.repo_file().display().to_string()));
    assert!(e.message.contains("`defect`"), "{}", e.message);
    assert!(!e.message.contains("clm.toml"), "{}", e.message);
    assert_eq!(
        e.to_string(),
        format!("{}: {}", f.repo_file().display(), e.message)
    );

    let e = Fixture::new(None, None)
        .load(&[("CLM_DEFECTS", "always")])
        .unwrap_err();
    assert_eq!(e.source.as_deref(), Some("CLM_DEFECTS=always"));

    let s = Fixture::new(None, None).load(&[]).unwrap();
    let e = s.get("colour").unwrap_err();
    assert_eq!(e.source, None);
    assert_eq!(e.to_string(), "unknown key `colour`");
}

#[test]
fn detectors_from_the_repository() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(
        None,
        Some("[[detectors]]\nname = \"machete\"\ncommand = [\"cargo\", \"machete\"]\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(
        s.detectors.value,
        vec![Detector {
            name: "machete".into(),
            command: vec!["cargo".into(), "machete".into()],
            hint: None,
            timeout: 600,
            requires: None,
        }]
    );
    assert_eq!(s.detectors.layer, Layer::File(f.repo_file()));
}

#[test]
fn detectors_keep_their_hint() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(
        None,
        Some("[[detectors]]\nname = \"m\"\ncommand = [\"m\"]\nhint = \"install m\"\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.detectors.value[0].hint.as_deref(), Some("install m"));
}

#[test]
fn detectors_default_to_none() {
    // req:config/repository-only@d2cdaefd
    let s = Fixture::new(None, Some("defects = \"off\"\n"))
        .load(&[])
        .unwrap();
    assert_eq!(s.detectors.value, vec![]);
    assert_eq!(s.detectors.layer, Layer::Default);
    let s = Fixture::new(None, None).load(&[]).unwrap();
    assert_eq!(s.detectors.layer, Layer::Default);
}

#[test]
fn detectors_in_the_user_file_fail() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(
        Some("defects = \"off\"\n[[detectors]]\nname = \"m\"\ncommand = [\"m\"]\n"),
        None,
    );
    let e = error(&f, &[]);
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `detectors`"), "{e}");
}

#[test]
fn detector_with_an_empty_command_fails() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(None, Some("[[detectors]]\nname = \"m\"\ncommand = []\n"));
    let e = error(&f, &[]);
    assert!(e.contains("clm.toml"), "{e}");
    assert!(
        e.contains("`detectors`: detector 1 has an empty command"),
        "{e}"
    );
}

#[test]
fn detector_with_an_empty_name_fails() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(
        None,
        Some("[[detectors]]\nname = \"\"\ncommand = [\"m\"]\n"),
    );
    let e = error(&f, &[]);
    assert!(e.contains("clm.toml"), "{e}");
    assert!(e.contains("`detectors`: detector 1 has no name"), "{e}");
}

#[test]
fn detector_without_a_name_fails() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(None, Some("[[detectors]]\ncommand = [\"m\"]\n"));
    let e = error(&f, &[]);
    assert!(e.contains("clm.toml"), "{e}");
    assert!(e.contains("`detectors`: detector 1 has no name"), "{e}");
}

#[test]
fn detector_with_an_unknown_key_fails() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(
        None,
        Some("[[detectors]]\nname = \"m\"\ncommand = [\"m\"]\ncmd = \"m\"\n"),
    );
    let e = error(&f, &[]);
    assert!(e.contains("clm.toml"), "{e}");
    assert!(e.contains("cmd"), "{e}");
}

#[test]
fn detectors_have_no_variable() {
    // req:config/repository-only@d2cdaefd
    let s = Fixture::new(None, None)
        .load(&[("CLM_DETECTORS", "m")])
        .unwrap();
    assert_eq!(s.detectors.value, vec![]);
    assert_eq!(s.detectors.layer, Layer::Default);
}

#[test]
fn get_detectors_gives_their_names() {
    // req:config/repository-only@d2cdaefd
    let f = Fixture::new(
        None,
        Some(
            "[[detectors]]\nname = \"a\"\ncommand = [\"a\"]\n\
             [[detectors]]\nname = \"b\"\ncommand = [\"b\"]\n",
        ),
    );
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("detectors").unwrap();
    assert_eq!(value, "a,b");
    assert_eq!(*layer, Layer::File(f.repo_file()));

    let s = Fixture::new(None, None).load(&[]).unwrap();
    let (value, layer) = s.get("detectors").unwrap();
    assert_eq!(value, "");
    assert_eq!(*layer, Layer::Default);
}

#[test]
fn layer_display_is_the_config_get_format() {
    // req:config/config-get@e2bb5643
    assert_eq!(Layer::Env("CLM_DEFECTS").to_string(), "env CLM_DEFECTS");
    assert_eq!(Layer::File("/r/clm.toml".into()).to_string(), "/r/clm.toml");
    assert_eq!(Layer::Default.to_string(), "default");
}

#[cfg(unix)]
#[test]
fn process_env_keeps_a_variable_that_is_not_utf8_set() {
    // req:config/invalid-settings@ec741034
    use std::os::unix::ffi::OsStrExt;
    // No other test in this binary reads the process environment.
    unsafe {
        std::env::set_var(
            "CLM_CONFIG_TEST_NOT_UTF8",
            std::ffi::OsStr::from_bytes(b"ask\xff"),
        );
        std::env::set_var("CLM_CONFIG_TEST_PLAIN", "off");
        std::env::remove_var("CLM_CONFIG_TEST_UNSET");
    }
    assert_eq!(
        clm_config::process_env("CLM_CONFIG_TEST_NOT_UTF8").as_deref(),
        Some("ask\u{fffd}")
    );
    assert_eq!(
        clm_config::process_env("CLM_CONFIG_TEST_PLAIN").as_deref(),
        Some("off")
    );
    assert_eq!(clm_config::process_env("CLM_CONFIG_TEST_UNSET"), None);
}

#[test]
fn host_defaults_to_github() {
    // req:config/host-setting@cbcd8366
    for repo in [None, Some("defects = \"off\"\n")] {
        let s = Fixture::new(None, repo).load(&[]).unwrap();
        assert_eq!(s.host.value, Host::Github);
        assert_eq!(s.host.layer, Layer::Default);
        let (value, layer) = s.get("host").unwrap();
        assert_eq!(value, "github");
        assert_eq!(*layer, Layer::Default);
    }
}

#[test]
fn host_from_the_repository() {
    // req:config/host-setting@cbcd8366
    for (text, want) in [
        ("none", Host::None),
        ("git", Host::Git),
        ("github", Host::Github),
        ("azure", Host::Azure),
    ] {
        let f = Fixture::new(None, Some(&format!("host = \"{text}\"\n")));
        let s = f.load(&[]).unwrap();
        assert_eq!(s.host.value, want);
        assert_eq!(s.host.layer, Layer::File(f.repo_file()));
        assert_eq!(s.get("host").unwrap().0, text);
    }
}

#[test]
fn unknown_host_fails() {
    // req:config/host-setting@cbcd8366
    for bad in ["host = \"gitlab\"\n", "host = 1\n"] {
        let f = Fixture::new(None, Some(bad));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()));
        assert!(e.message.contains("host"), "{}", e.message);
    }
}

#[test]
fn host_in_the_user_file_fails() {
    // req:config/host-setting@cbcd8366
    let e = error(&Fixture::new(Some("host = \"none\"\n"), None), &[]);
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `host`"), "{e}");
}

#[test]
fn host_has_no_variable() {
    // req:config/host-setting@cbcd8366
    let s = Fixture::new(None, None)
        .load(&[("CLM_HOST", "none")])
        .unwrap();
    assert_eq!(s.host.value, Host::Github);
    assert_eq!(s.host.layer, Layer::Default);
}

#[test]
fn approval_defaults_to_any() {
    // req:config/approval-setting@ee7ad72c
    let s = Fixture::new(None, None).load(&[]).unwrap();
    assert_eq!(s.approval.value, Approval::Any);
    let (value, layer) = s.get("approval").unwrap();
    assert_eq!(value, "any");
    assert_eq!(*layer, Layer::Default);
}

#[test]
fn approval_from_the_repository() {
    // req:config/approval-setting@ee7ad72c
    for (text, want) in [("any", Approval::Any), ("other", Approval::Other)] {
        let f = Fixture::new(None, Some(&format!("approval = \"{text}\"\n")));
        let s = f.load(&[]).unwrap();
        assert_eq!(s.approval.value, want);
        assert_eq!(s.approval.layer, Layer::File(f.repo_file()));
        assert_eq!(s.get("approval").unwrap().0, text);
    }
}

#[test]
fn unknown_approval_fails() {
    // req:config/approval-setting@ee7ad72c
    let f = Fixture::new(None, Some("approval = \"self\"\n"));
    let e = f.load(&[]).unwrap_err();
    assert_eq!(e.source, Some(f.repo_file().display().to_string()));
    assert!(e.message.contains("approval"), "{}", e.message);
}

#[test]
fn approval_in_the_user_file_fails() {
    // req:config/approval-setting@ee7ad72c
    let e = error(&Fixture::new(Some("approval = \"any\"\n"), None), &[]);
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `approval`"), "{e}");
}

#[test]
fn approval_has_no_variable() {
    // req:config/approval-setting@ee7ad72c
    let s = Fixture::new(None, None)
        .load(&[("CLM_APPROVAL", "other")])
        .unwrap();
    assert_eq!(s.approval.value, Approval::Any);
    assert_eq!(s.approval.layer, Layer::Default);
}

#[test]
fn land_defaults_to_no_command() {
    // req:config/land-check-setting@9a71815d
    for repo in [None, Some("defects = \"off\"\n")] {
        let s = Fixture::new(None, repo).load(&[]).unwrap();
        assert_eq!(s.land.value, None);
        assert_eq!(s.land.layer, Layer::Default);
        let (value, layer) = s.get("land").unwrap();
        assert_eq!(value, "");
        assert_eq!(*layer, Layer::Default);
    }
}

#[test]
fn land_check_from_the_repository() {
    // req:config/land-check-setting@9a71815d
    let f = Fixture::new(None, Some("[land]\ncheck = \"scripts/clm-check.sh\"\n"));
    let s = f.load(&[]).unwrap();
    assert_eq!(
        s.land.value,
        Some(Land {
            check: "scripts/clm-check.sh".into(),
            target_cap_gb: None
        })
    );
    assert_eq!(s.land.layer, Layer::File(f.repo_file()));
    let (value, layer) = s.get("land").unwrap();
    assert_eq!(value, "check=scripts/clm-check.sh");
    assert_eq!(*layer, Layer::File(f.repo_file()));
}

#[test]
fn land_cap_set_prints_and_loads() {
    // req:config/land-check-setting@9a71815d
    let f = Fixture::new(
        None,
        Some("[land]\ncheck = \"scripts/clm-check.sh\"\ntarget_cap_gb = 4\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.land.value.as_ref().unwrap().cap_gb(), 4);
    let (value, layer) = s.get("land").unwrap();
    assert_eq!(value, "check=scripts/clm-check.sh,target_cap_gb=4");
    assert_eq!(*layer, Layer::File(f.repo_file()));
}

#[test]
fn land_cap_defaults_to_fifteen() {
    // req:config/land-check-setting@9a71815d
    let f = Fixture::new(None, Some("[land]\ncheck = \"scripts/clm-check.sh\"\n"));
    let s = f.load(&[]).unwrap();
    assert_eq!(s.land.value.unwrap().cap_gb(), 15);
}

#[test]
fn bad_land_fails() {
    // req:config/land-check-setting@9a71815d
    for bad in [
        "[land]\ncheck = \"\"\n",
        "[land]\n",
        "[land]\ncheck = 1\n",
        "[land]\ncheck = [\"a\"]\n",
        "[land]\ncheck = \"a\"\ncommand = \"b\"\n",
        "land = \"a\"\n",
        "[land]\ncheck = \"a\"\ntarget_cap_gb = 0\n",
        "[land]\ncheck = \"a\"\ntarget_cap_gb = -1\n",
        "[land]\ncheck = \"a\"\ntarget_cap_gb = \"4\"\n",
        "[land]\ncheck = \"a\"\ntarget_cap_gb = 1.5\n",
        "[land]\ntarget_cap_gb = 4\n",
    ] {
        let f = Fixture::new(None, Some(bad));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()), "{bad}");
        assert!(e.message.contains("`land`"), "{bad}: {}", e.message);
    }
}

#[test]
fn land_in_the_user_file_fails() {
    // req:config/land-check-setting@9a71815d
    let e = error(&Fixture::new(Some("[land]\ncheck = \"true\"\n"), None), &[]);
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `land`"), "{e}");
}

#[test]
fn land_has_no_variable() {
    // req:config/land-check-setting@9a71815d
    let s = Fixture::new(None, None)
        .load(&[("CLM_LAND", "true")])
        .unwrap();
    assert_eq!(s.land.value, None);
    assert_eq!(s.land.layer, Layer::Default);
}

#[test]
fn writer_from_the_variable() {
    // req:config/writer-setting@16b0cd7d
    let f = Fixture::new(Some("writer = \"user\"\n"), None);
    let s = f.load(&[("CLM_WRITER", "christopher")]).unwrap();
    assert_eq!(s.writer.value.as_deref(), Some("christopher"));
    assert_eq!(s.writer.layer, Layer::Env("CLM_WRITER"));
    let (value, layer) = s.get("writer").unwrap();
    assert_eq!(value, "christopher");
    assert_eq!(layer.to_string(), "env CLM_WRITER");
}

#[test]
fn writer_from_the_user_file() {
    // req:config/writer-setting@16b0cd7d
    let f = Fixture::new(
        Some("writer = \"alice\"\n"),
        Some("defects = \"backlog\"\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.writer.value.as_deref(), Some("alice"));
    assert_eq!(s.writer.layer, Layer::File(f.user_file()));
}

#[test]
fn writer_from_the_repository_fails() {
    // req:config/writer-setting@16b0cd7d
    let f = Fixture::new(
        Some("writer = \"alice\"\n"),
        Some("writer = \"christopher\"\n"),
    );
    for vars in [&[][..], &[("CLM_WRITER", "christopher")][..]] {
        let e = f.load(vars).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()));
        assert!(
            e.message.contains("unknown field `writer`"),
            "{}",
            e.message
        );
    }
}

#[test]
fn writer_not_a_slug_fails() {
    // req:config/writer-setting@16b0cd7d
    for bad in ["Christopher L", "", "christopher_l", "Christopher"] {
        let e = Fixture::new(None, None)
            .load(&[("CLM_WRITER", bad)])
            .unwrap_err();
        assert_eq!(e.source, Some(format!("CLM_WRITER={bad}")));
        assert!(e.message.contains("`writer`"), "{}", e.message);
    }
    for bad in ["", "Christopher L"] {
        let f = Fixture::new(Some(&format!("writer = {bad:?}\n")), None);
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.user_file().display().to_string()));
        assert!(e.message.contains("`writer`"), "{}", e.message);
    }
}

#[test]
fn writer_slug_accepts_digits_and_hyphens() {
    // req:config/writer-setting@16b0cd7d
    let s = Fixture::new(None, None)
        .load(&[("CLM_WRITER", "christopher-2")])
        .unwrap();
    assert_eq!(s.writer.value.as_deref(), Some("christopher-2"));
}

#[test]
fn writer_unset_is_left_to_the_caller() {
    // req:config/writer-setting@16b0cd7d
    let s = Fixture::new(None, None).load(&[]).unwrap();
    assert_eq!(s.writer.value, None);
    assert_eq!(s.writer.layer, Layer::Default);
    let (value, layer) = s.get("writer").unwrap();
    assert_eq!(value, "(git user.name)");
    assert_eq!(*layer, Layer::Default);
}

#[test]
fn local_over_repository() {
    // req:config/layers@cf14f119
    let f = Fixture::new(None, Some("defects = \"backlog\"\nhost = \"github\"\n"))
        .local("defects = \"off\"\n");
    let s = f.load(&[]).unwrap();
    assert_eq!(s.defects.value, Defects::Off);
    assert_eq!(s.defects.layer, Layer::File(f.local_file()));
    assert_eq!(s.host.value, Host::Github);
    assert_eq!(s.host.layer, Layer::File(f.repo_file()));
}

#[test]
fn variable_over_local() {
    // req:config/layers@cf14f119
    let f = files().local("defects = \"off\"\nautosync = true\n");
    let s = f
        .load(&[("CLM_DEFECTS", "ask"), ("CLM_AUTOSYNC", "0")])
        .unwrap();
    assert_eq!(s.defects.value, Defects::Ask);
    assert_eq!(s.defects.layer, Layer::Env("CLM_DEFECTS"));
    assert!(!s.autosync.value);
    assert_eq!(s.autosync.layer, Layer::Env("CLM_AUTOSYNC"));
}

#[test]
fn local_over_user() {
    // req:config/layers@cf14f119
    let f = Fixture::new(Some("autosync = false\n"), None).local("autosync = true\n");
    let s = f.load(&[]).unwrap();
    assert!(s.autosync.value);
    assert_eq!(s.autosync.layer, Layer::File(f.local_file()));
}

#[test]
fn tables_dont_merge() {
    // req:config/layers@cf14f119
    let f = Fixture::new(
        None,
        Some(
            "[component_size]\nspec_tokens = 20000\ncode_tokens = 50000\n\
             [land]\ncheck = \"a\"\n\
             [[detectors]]\nname = \"a\"\ncommand = [\"a\"]\n\
             [[detectors]]\nname = \"b\"\ncommand = [\"b\"]\n",
        ),
    )
    .local(
        "[component_size]\ncode_tokens = 60000\n\
         [[detectors]]\nname = \"c\"\ncommand = [\"c\"]\n",
    );
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("component_size").unwrap();
    assert_eq!(value, "spec_tokens=12000,code_tokens=60000");
    assert_eq!(*layer, Layer::File(f.local_file()));
    let (value, layer) = s.get("detectors").unwrap();
    assert_eq!(value, "c");
    assert_eq!(*layer, Layer::File(f.local_file()));
    let (value, layer) = s.get("land").unwrap();
    assert_eq!(value, "check=a");
    assert_eq!(*layer, Layer::File(f.repo_file()));
}

#[test]
fn local_file_takes_every_key() {
    // req:config/local-settings@6ef008e6
    let f = Fixture::new(None, Some("[land]\ncheck = \"scripts/clm-check.sh\"\n")).local(
        "autosync = false\nmax_builds = 9\ndefects = \"off\"\nwriter = \"bob\"\nhost = \"git\"\n\
         approval = \"other\"\n[land]\ncheck = \"scripts/quick.sh\"\n\
         [component_size]\nspec_tokens = 1\n\
         [[detectors]]\nname = \"m\"\ncommand = [\"m\"]\n",
    );
    let s = f.load(&[]).unwrap();
    for (key, want) in [
        ("autosync", "false"),
        ("max_builds", "9"),
        ("defects", "off"),
        ("writer", "bob"),
        ("host", "git"),
        ("approval", "other"),
        ("land", "check=scripts/quick.sh"),
        ("component_size", "spec_tokens=1,code_tokens=40000"),
        ("detectors", "m"),
    ] {
        let (value, layer) = s.get(key).unwrap();
        assert_eq!(value, want, "{key}");
        assert_eq!(*layer, Layer::File(f.local_file()), "{key}");
    }
}

#[test]
fn invalid_local_file_names_it() {
    // req:config/local-settings@6ef008e6
    for (text, key) in [
        ("defect = \"off\"\n", "defect"),
        ("defects = \"always\"\n", "defects"),
        ("defects = \n", "defects"),
        ("writer = \"Bob\"\n", "writer"),
    ] {
        let f = Fixture::new(None, Some("defects = \"backlog\"\n")).local(text);
        let e = f.load(&[]).unwrap_err();
        assert_eq!(
            e.source,
            Some(f.local_file().display().to_string()),
            "{text}"
        );
        assert!(e.to_string().contains("clm.local.toml"), "{e}");
        assert!(e.message.contains(key), "{text}: {}", e.message);
    }
}

#[test]
fn no_local_file_adds_no_layer() {
    // req:config/local-settings@6ef008e6
    let f = Fixture::new(None, Some("defects = \"backlog\"\n"));
    let s = f.load(&[]).unwrap();
    assert_eq!(s.defects.value, Defects::Backlog);
    assert_eq!(s.defects.layer, Layer::File(f.repo_file()));
}

#[test]
fn repository_only_keys_set_locally_fail_naming_the_local_file() {
    // req:config/host-setting@cbcd8366
    // req:config/approval-setting@ee7ad72c
    // req:config/land-check-setting@9a71815d
    // req:config/component-size-setting@8f8238ab
    // req:config/repository-only@d2cdaefd
    for (text, key) in [
        ("host = \"gitlab\"\n", "host"),
        ("approval = \"self\"\n", "approval"),
        ("[land]\ncheck = \"\"\n", "`land`"),
        ("[component_size]\nspec_tokens = 0\n", "`component_size`"),
        ("[[detectors]]\nname = \"m\"\ncommand = []\n", "`detectors`"),
    ] {
        let f = Fixture::new(None, Some("host = \"github\"\n")).local(text);
        let e = f.load(&[]).unwrap_err();
        assert_eq!(
            e.source,
            Some(f.local_file().display().to_string()),
            "{text}"
        );
        assert!(e.message.contains(key), "{text}: {}", e.message);
    }
}

#[test]
fn writer_from_the_local_file() {
    // req:config/writer-setting@16b0cd7d
    let f = Fixture::new(Some("writer = \"alice\"\n"), None).local("writer = \"bob\"\n");
    let s = f.load(&[]).unwrap();
    assert_eq!(s.writer.value.as_deref(), Some("bob"));
    assert_eq!(s.writer.layer, Layer::File(f.local_file()));
    let s = f.load(&[("CLM_WRITER", "christopher")]).unwrap();
    assert_eq!(s.writer.layer, Layer::Env("CLM_WRITER"));
}

fn git(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A git repository at `main/` with a commit, and a linked worktree of it at `linked/`.
fn worktrees() -> (TempDir, PathBuf, PathBuf) {
    let dir = TempDir::new().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let main = base.join("main");
    fs::create_dir_all(main.join("tool")).unwrap();
    git(&main, &["init", "-q"]);
    fs::write(main.join("tool/clm.toml"), "defects = \"backlog\"\n").unwrap();
    git(&main, &["add", "."]);
    git(
        &main,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "init",
        ],
    );
    let linked = base.join("linked");
    git(&main, &["worktree", "add", "-q", linked.to_str().unwrap()]);
    (dir, main, linked)
}

fn no_user_load(root: &Path) -> Settings {
    load_with(Some(root), &HashMap::new()).unwrap()
}

#[test]
fn local_file_read_from_a_linked_worktree() {
    // req:config/local-settings@6ef008e6
    let (_dir, main, linked) = worktrees();
    fs::write(main.join("clm.local.toml"), "defects = \"off\"\n").unwrap();
    // A file the linked worktree's own directory holds is never read.
    fs::write(linked.join("clm.local.toml"), "defects = \"ask\"\n").unwrap();
    for root in [&main, &linked] {
        let s = no_user_load(root);
        let (value, layer) = s.get("defects").unwrap();
        assert_eq!(value, "off");
        assert_eq!(*layer, Layer::File(main.join("clm.local.toml")));
    }
}

#[test]
fn local_file_in_a_subdirectory_repository() {
    // req:config/local-settings@6ef008e6
    let (_dir, main, linked) = worktrees();
    fs::write(main.join("tool/clm.local.toml"), "defects = \"off\"\n").unwrap();
    fs::write(main.join("clm.local.toml"), "defects = \"ask\"\n").unwrap();
    let s = no_user_load(&linked.join("tool"));
    assert_eq!(s.defects.value, Defects::Off);
    assert_eq!(
        s.defects.layer,
        Layer::File(main.join("tool/clm.local.toml"))
    );
}

#[test]
fn unreadable_gitdir_reads_beside_the_root() {
    // req:config/local-settings@6ef008e6
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("r");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("clm.local.toml"), "defects = \"off\"\n").unwrap();
    for git_file in ["gitdir: /no/such/dir\n", "not a gitdir line\n"] {
        fs::write(root.join(".git"), git_file).unwrap();
        let s = no_user_load(&root);
        assert_eq!(s.defects.value, Defects::Off, "{git_file}");
        assert_eq!(s.defects.layer, Layer::File(root.join("clm.local.toml")));
    }
}

#[test]
fn bare_common_dir_has_no_local_layer() {
    // req:config/local-settings@6ef008e6
    let dir = TempDir::new().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let wt = base.join("repo.git/worktrees/w");
    fs::create_dir_all(&wt).unwrap();
    fs::write(wt.join("commondir"), "../..\n").unwrap();
    let root = base.join("w");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".git"), format!("gitdir: {}\n", wt.display())).unwrap();
    fs::write(root.join("clm.local.toml"), "defects = \"off\"\n").unwrap();
    fs::write(base.join("clm.local.toml"), "defects = \"off\"\n").unwrap();
    let s = no_user_load(&root);
    assert_eq!(s.defects.layer, Layer::Default);
}

#[test]
fn max_builds_defaults_to_3() {
    // req:config/max-builds-setting@11205a64
    let s = Fixture::new(None, None).load(&[]).unwrap();
    assert_eq!(s.max_builds.value, 3);
    let (value, layer) = s.get("max_builds").unwrap();
    assert_eq!(value, "3");
    assert_eq!(*layer, Layer::Default);
}

#[test]
fn max_builds_in_the_layers_order() {
    // req:config/max-builds-setting@11205a64
    let f = Fixture::new(Some("max_builds = 7\n"), None);
    let s = f.load(&[]).unwrap();
    assert_eq!(s.max_builds.value, 7);
    assert_eq!(s.max_builds.layer, Layer::File(f.user_file()));

    let f = Fixture::new(Some("max_builds = 7\n"), Some("max_builds = 2\n"));
    let s = f.load(&[]).unwrap();
    assert_eq!(s.max_builds.value, 2);
    assert_eq!(s.max_builds.layer, Layer::File(f.repo_file()));

    let f = f.local("max_builds = 5\n");
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("max_builds").unwrap();
    assert_eq!(value, "5");
    assert_eq!(*layer, Layer::File(f.local_file()));

    let s = f.load(&[("CLM_MAX_BUILDS", "1")]).unwrap();
    let (value, layer) = s.get("max_builds").unwrap();
    assert_eq!(value, "1");
    assert_eq!(layer.to_string(), "env CLM_MAX_BUILDS");
}

#[test]
fn max_builds_must_be_a_whole_number_of_at_least_1() {
    // req:config/max-builds-setting@11205a64
    for bad in ["0", "-1", "1.5", "\"3\"", "true"] {
        let text = format!("max_builds = {bad}\n");
        for f in [
            Fixture::new(None, None).local(&text),
            Fixture::new(None, Some(&text)),
        ] {
            let e = f.load(&[]).unwrap_err();
            let file = if f.local_file().exists() {
                f.local_file()
            } else {
                f.repo_file()
            };
            assert_eq!(e.source, Some(file.display().to_string()), "{bad}");
            assert!(e.message.contains("`max_builds`"), "{bad}: {}", e.message);
        }
        let f = Fixture::new(Some(&text), None);
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.user_file().display().to_string()), "{bad}");
        assert!(e.message.contains("`max_builds`"), "{bad}: {}", e.message);
    }
}

#[test]
fn bad_max_builds_variable() {
    // req:config/max-builds-setting@11205a64
    for bad in ["many", "0", "", "-1", "+2", " 2", "1.0"] {
        let e = Fixture::new(None, None)
            .load(&[("CLM_MAX_BUILDS", bad)])
            .unwrap_err();
        assert_eq!(e.source, Some(format!("CLM_MAX_BUILDS={bad}")), "{bad}");
        assert!(e.message.contains("`max_builds`"), "{bad}: {}", e.message);
    }
    let s = Fixture::new(None, None)
        .load(&[("CLM_MAX_BUILDS", "12")])
        .unwrap();
    assert_eq!(s.max_builds.value, 12);
}

#[test]
fn this_repository_ignores_local_and_session_files() {
    // req:config/local-settings-ignored@274677aa
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.gitignore");
    let text = fs::read_to_string(path).unwrap();
    for line in ["clm.local.toml", ".clm/", ".claude/worktrees/"] {
        assert!(text.lines().any(|l| l == line), "{line} in {text}");
    }
}

#[test]
fn relative_gitdir_finds_the_main_worktree() {
    // req:config/local-settings@6ef008e6
    let (_dir, main, linked) = worktrees();
    let gitdir = fs::read_to_string(linked.join(".git")).unwrap();
    let abs = Path::new(gitdir.trim().strip_prefix("gitdir: ").unwrap());
    let rel = Path::new("..").join(abs.strip_prefix(linked.parent().unwrap()).unwrap());
    fs::write(linked.join(".git"), format!("gitdir: {}\n", rel.display())).unwrap();
    fs::write(main.join("clm.local.toml"), "defects = \"off\"\n").unwrap();
    let s = no_user_load(&linked);
    assert_eq!(s.defects.layer, Layer::File(main.join("clm.local.toml")));
}

#[test]
fn max_builds_takes_any_whole_number() {
    // req:config/max-builds-setting@11205a64
    let s = Fixture::new(None, Some("max_builds = 5000000000\n"))
        .load(&[("CLM_MAX_BUILDS", "6000000000")])
        .unwrap();
    assert_eq!(s.max_builds.value, 6_000_000_000);
    let s = Fixture::new(None, Some("max_builds = 5000000000\n"))
        .load(&[])
        .unwrap();
    assert_eq!(s.max_builds.value, 5_000_000_000);
}

#[test]
fn affected_defaults_to_no_glob() {
    // req:config/affected-setting@7b51c4ec
    let s = Fixture::new(None, None).load(&[]).unwrap();
    assert_eq!(s.affected.value, None);
    assert_eq!(s.affected.layer, Layer::Default);
    let (value, layer) = s.get("affected").unwrap();
    assert_eq!(value, "");
    assert_eq!(*layer, Layer::Default);
}

#[test]
fn affected_from_the_repository() {
    // req:config/affected-setting@7b51c4ec
    let f = Fixture::new(
        None,
        Some("[affected]\nall = [\"Cargo.lock\", \"rust-toolchain.toml\"]\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(
        s.affected.value,
        Some(Affected {
            all: vec!["Cargo.lock".into(), "rust-toolchain.toml".into()]
        })
    );
    let (value, layer) = s.get("affected").unwrap();
    assert_eq!(value, "all=Cargo.lock,rust-toolchain.toml");
    assert_eq!(*layer, Layer::File(f.repo_file()));
}

#[test]
fn affected_with_no_globs() {
    // req:config/affected-setting@7b51c4ec
    let f = Fixture::new(None, Some("[affected]\nall = []\n"));
    let s = f.load(&[]).unwrap();
    assert_eq!(s.affected.value, Some(Affected { all: vec![] }));
    let (value, layer) = s.get("affected").unwrap();
    assert_eq!(value, "all=");
    assert_eq!(*layer, Layer::File(f.repo_file()));
}

#[test]
fn bad_affected_fails() {
    // req:config/affected-setting@7b51c4ec
    for bad in [
        "[affected]\nall = [\"\"]\n",
        "[affected]\n",
        "[affected]\nall = \"Cargo.lock\"\n",
        "[affected]\nall = [1]\n",
        "[affected]\nall = []\nsome = []\n",
        "affected = [\"Cargo.lock\"]\n",
    ] {
        let f = Fixture::new(None, Some(bad));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()), "{bad}");
        assert!(e.message.contains("`affected`"), "{bad}: {}", e.message);
        let f = Fixture::new(None, None).local(bad);
        let e = f.load(&[]).unwrap_err();
        assert_eq!(
            e.source,
            Some(f.local_file().display().to_string()),
            "{bad}"
        );
        assert!(e.message.contains("`affected`"), "{bad}: {}", e.message);
    }
}

#[test]
fn affected_in_the_user_file_fails() {
    // req:config/affected-setting@7b51c4ec
    let e = error(&Fixture::new(Some("[affected]\nall = []\n"), None), &[]);
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `affected`"), "{e}");
}

#[test]
fn affected_comes_whole_from_the_local_file() {
    // req:config/affected-setting@7b51c4ec
    let f = Fixture::new(None, Some("[affected]\nall = [\"Cargo.lock\"]\n"))
        .local("[affected]\nall = [\"x\"]\n");
    let s = f.load(&[]).unwrap();
    assert_eq!(
        s.affected.value,
        Some(Affected {
            all: vec!["x".into()]
        })
    );
    assert_eq!(s.affected.layer, Layer::File(f.local_file()));
}

#[test]
fn detector_timeout_defaults_to_600() {
    // req:config/detector-timeout-setting@2348dcce
    let f = Fixture::new(
        None,
        Some("[[detectors]]\nname = \"machete\"\ncommand = [\"cargo-machete\"]\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.detectors.value[0].name, "machete");
    assert_eq!(s.detectors.value[0].timeout, 600);
}

#[test]
fn detector_timeout_set() {
    // req:config/detector-timeout-setting@2348dcce
    let f = Fixture::new(
        None,
        Some("[[detectors]]\nname = \"slow\"\ncommand = [\"sleep\", \"5\"]\ntimeout = 2\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.detectors.value[0].name, "slow");
    assert_eq!(s.detectors.value[0].timeout, 2);
}

#[test]
fn detector_timeout_must_be_a_whole_number_of_at_least_1() {
    // req:config/detector-timeout-setting@2348dcce
    for bad in ["0", "-1", "1.5", "\"5\"", "[5]"] {
        let text =
            format!("[[detectors]]\nname = \"slow\"\ncommand = [\"sleep\"]\ntimeout = {bad}\n");
        let f = Fixture::new(None, None).local(&text);
        let e = f.load(&[]).unwrap_err();
        assert_eq!(
            e.source,
            Some(f.local_file().display().to_string()),
            "{bad}"
        );
        assert!(e.message.contains("`detectors`"), "{bad}: {}", e.message);
        let f = Fixture::new(None, Some(&text));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()), "{bad}");
        assert!(e.message.contains("`detectors`"), "{bad}: {}", e.message);
    }
}

#[test]
fn detector_requires_set() {
    // req:config/detector-requires-setting@5f78cf14
    let f = Fixture::new(
        None,
        Some(
            "[[detectors]]\nname = \"knip\"\ncommand = [\"node_modules/.bin/knip\"]\nrequires = \"package.json\"\n",
        ),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.detectors.value[0].name, "knip");
    assert_eq!(
        s.detectors.value[0].requires.as_deref(),
        Some("package.json")
    );
}

#[test]
fn detector_requires_left_out() {
    // req:config/detector-requires-setting@5f78cf14
    let f = Fixture::new(
        None,
        Some("[[detectors]]\nname = \"machete\"\ncommand = [\"cargo-machete\"]\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.detectors.value[0].requires, None);
}

#[test]
fn detector_requires_must_be_a_non_empty_string() {
    // req:config/detector-requires-setting@5f78cf14
    for bad in ["\"\"", "5", "[\"a\"]", "true"] {
        let text = format!("[[detectors]]\nname = \"k\"\ncommand = [\"k\"]\nrequires = {bad}\n");
        let f = Fixture::new(None, None).local(&text);
        let e = f.load(&[]).unwrap_err();
        assert_eq!(
            e.source,
            Some(f.local_file().display().to_string()),
            "{bad}"
        );
        assert!(e.message.contains("`detectors`"), "{bad}: {}", e.message);
        let f = Fixture::new(None, Some(&text));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()), "{bad}");
    }
}

#[test]
fn health_defaults() {
    // req:config/health-setting@69ee7c01
    for repo in [None, Some("defects = \"off\"\n")] {
        let s = Fixture::new(None, repo).load(&[]).unwrap();
        assert_eq!(
            s.health.value,
            Health {
                reconsider: 3,
                scenarios: 6,
                exceptions: 4,
                stale_days: 30,
                lcov: None,
            }
        );
        assert_eq!(s.health.layer, Layer::Default);
        let (value, layer) = s.get("health").unwrap();
        assert_eq!(value, "reconsider=3,scenarios=6,exceptions=4,stale_days=30");
        assert_eq!(*layer, Layer::Default);
    }
}

#[test]
fn health_two_keys_set() {
    // req:config/health-setting@69ee7c01
    let f = Fixture::new(
        None,
        Some("[health]\nreconsider = 5\nlcov = \"target/lcov.info\"\n"),
    );
    let s = f.load(&[]).unwrap();
    assert_eq!(s.health.value.reconsider, 5);
    assert_eq!(s.health.value.lcov.as_deref(), Some("target/lcov.info"));
    let (value, layer) = s.get("health").unwrap();
    assert_eq!(
        value,
        "reconsider=5,scenarios=6,exceptions=4,stale_days=30,lcov=target/lcov.info"
    );
    assert_eq!(*layer, Layer::File(f.repo_file()));
}

#[test]
fn health_every_count_is_read() {
    // req:config/health-setting@69ee7c01
    let f = Fixture::new(
        None,
        Some("[health]\nreconsider = 1\nscenarios = 2\nexceptions = 7\nstale_days = 9\n"),
    );
    let (value, _) = f.load(&[]).unwrap().get("health").unwrap();
    assert_eq!(value, "reconsider=1,scenarios=2,exceptions=7,stale_days=9");
}

#[test]
fn health_counts_must_be_positive_integers() {
    // req:config/health-setting@69ee7c01
    for key in ["reconsider", "scenarios", "exceptions", "stale_days"] {
        for bad in ["0", "-1", "1.5", "\"3\""] {
            let f = Fixture::new(None, Some(&format!("[health]\n{key} = {bad}\n")));
            let e = f.load(&[]).unwrap_err();
            assert_eq!(e.source, Some(f.repo_file().display().to_string()));
            assert!(e.message.contains("`health`"), "{}", e.message);
            assert!(e.message.contains(key), "{}", e.message);
        }
    }
}

#[test]
fn health_lcov_must_be_a_string() {
    // req:config/health-setting@69ee7c01
    let f = Fixture::new(None, Some("[health]\nlcov = 5\n"));
    let e = f.load(&[]).unwrap_err();
    assert_eq!(e.source, Some(f.repo_file().display().to_string()));
    assert!(e.message.contains("`health`"), "{}", e.message);
    assert!(e.message.contains("lcov"), "{}", e.message);
}

#[test]
fn health_lcov_must_be_a_path_under_the_root() {
    // req:config/health-setting@69ee7c01
    // Written as TOML basic strings, so `\t` is a tab and `\u0001` a control character.
    for bad in [
        r#""""#,
        r#"".""#,
        r#""./.""#,
        r#""/tmp/lcov.info""#,
        r#""../other/lcov.info""#,
        r#""target/../lcov.info""#,
        r#""cov=a,b.info""#,
        r#""cov=info""#,
        r#""cov,info""#,
        r#""cov\tinfo""#,
        r#""cov\u0001info""#,
    ] {
        let f = Fixture::new(None, Some(&format!("[health]\nlcov = {bad}\n")));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()), "{bad}");
        assert!(e.message.contains("`health`"), "{bad}: {}", e.message);
        assert!(e.message.contains("lcov"), "{bad}: {}", e.message);
        let f = Fixture::new(None, None).local(&format!("[health]\nlcov = {bad}\n"));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(
            e.source,
            Some(f.local_file().display().to_string()),
            "{bad}"
        );
        assert!(e.message.contains("`health`"), "{bad}: {}", e.message);
        assert!(e.message.contains("lcov"), "{bad}: {}", e.message);
    }
    for good in ["target/lcov.info", "./target/lcov.info", "lcov.info"] {
        let f = Fixture::new(None, Some(&format!("[health]\nlcov = \"{good}\"\n")));
        let s = f.load(&[]).unwrap();
        assert_eq!(s.health.value.lcov.as_deref(), Some(good));
    }
}

#[test]
fn health_must_be_a_table() {
    // req:config/health-setting@69ee7c01
    let f = Fixture::new(None, Some("health = 5\n"));
    let e = f.load(&[]).unwrap_err();
    assert_eq!(e.source, Some(f.repo_file().display().to_string()));
    assert!(e.message.contains("`health`"), "{}", e.message);
    assert!(e.message.contains("not 5"), "{}", e.message);
}

#[test]
fn health_unknown_key_fails() {
    // req:config/health-setting@69ee7c01
    let f = Fixture::new(None, Some("[health]\nscore = 5\n"));
    let e = f.load(&[]).unwrap_err();
    assert_eq!(e.source, Some(f.repo_file().display().to_string()));
    assert!(e.message.contains("`health`"), "{}", e.message);
    assert!(e.message.contains("`score`"), "{}", e.message);
}

#[test]
fn health_in_the_user_file_fails() {
    // req:config/health-setting@69ee7c01
    let e = error(&Fixture::new(Some("[health]\nreconsider = 1\n"), None), &[]);
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `health`"), "{e}");
}

#[test]
fn health_from_the_local_file_is_whole() {
    // req:config/health-setting@69ee7c01
    let f = Fixture::new(None, Some("[health]\nscenarios = 9\nreconsider = 8\n"))
        .local("[health]\nreconsider = 2\n");
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("health").unwrap();
    assert_eq!(value, "reconsider=2,scenarios=6,exceptions=4,stale_days=30");
    assert_eq!(*layer, Layer::File(f.local_file()));
}

#[test]
fn health_invalid_in_the_local_file_names_it() {
    // req:config/health-setting@69ee7c01
    let f = Fixture::new(None, None).local("[health]\nstale_days = 0\n");
    let e = f.load(&[]).unwrap_err();
    assert_eq!(e.source, Some(f.local_file().display().to_string()));
    assert!(e.message.contains("`health`"), "{}", e.message);
}

#[test]
fn effort_setting_default() {
    // req:config/effort-setting@70fef850
    let s = Fixture::new(None, None).load(&[]).unwrap();
    assert_eq!(s.effort.value.workaround_flag, 3);
    let (value, layer) = s.get("effort").unwrap();
    assert_eq!(value, "workaround_flag=3");
    assert_eq!(*layer, Layer::Default);
}

#[test]
fn effort_setting_from_file() {
    // req:config/effort-setting@70fef850
    let f = Fixture::new(None, Some("[effort]\nworkaround_flag = 2\n"));
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("effort").unwrap();
    assert_eq!(value, "workaround_flag=2");
    assert_eq!(*layer, Layer::File(f.repo_file()));
    let f = Fixture::new(None, Some("[effort]\nworkaround_flag = 2\n")).local("[effort]\n");
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("effort").unwrap();
    assert_eq!(value, "workaround_flag=3");
    assert_eq!(*layer, Layer::File(f.local_file()));
}

#[test]
fn effort_setting_not_positive() {
    // req:config/effort-setting@70fef850
    for text in [
        "[effort]\nworkaround_flag = 0\n",
        "[effort]\nworkaround_flag = -1\n",
        "[effort]\nworkaround_flag = \"3\"\n",
        "[effort]\nflag = 3\n",
        "effort = 3\n",
    ] {
        let f = Fixture::new(None, Some(text));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(
            e.source,
            Some(f.repo_file().display().to_string()),
            "{text}"
        );
        assert!(e.message.contains("`effort`"), "{text}: {}", e.message);
    }
}

#[test]
fn effort_setting_from_user_file() {
    // req:config/effort-setting@70fef850
    let e = error(
        &Fixture::new(Some("[effort]\nworkaround_flag = 1\n"), None),
        &[],
    );
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `effort`"), "{e}");
}

#[test]
fn component_size_the_new_name() {
    // req:config/component-size-setting@8f8238ab
    let f = Fixture::new(None, Some("[component_size]\ncode_tokens = 60000\n"));
    let s = f.load(&[]).unwrap();
    let (value, layer) = s.get("component_size").unwrap();
    assert_eq!(value, "spec_tokens=12000,code_tokens=60000");
    assert_eq!(*layer, Layer::File(f.repo_file()));
}

#[test]
fn component_size_defaults() {
    // req:config/component-size-setting@8f8238ab
    let s = Fixture::new(None, None).load(&[]).unwrap();
    let (value, layer) = s.get("component_size").unwrap();
    assert_eq!(value, "spec_tokens=12000,code_tokens=40000");
    assert_eq!(*layer, Layer::Default);
}

#[test]
fn component_size_old_name_refused_in_either_file() {
    // req:config/component-size-setting@8f8238ab
    for (text, local) in [
        ("[node_size]\nspec_tokens = 20000\n", false),
        ("node_size = 5\n", false),
        ("[node_size]\nspec_tokens = 20000\n", true),
        (
            "[component_size]\ncode_tokens = 1\n[node_size]\nspec_tokens = 1\n",
            false,
        ),
    ] {
        let f = if local {
            Fixture::new(None, Some("host = \"github\"\n")).local(text)
        } else {
            Fixture::new(None, Some(text))
        };
        let e = f.load(&[]).unwrap_err();
        let file = if local { f.local_file() } else { f.repo_file() };
        assert_eq!(e.source, Some(file.display().to_string()), "{text}");
        assert!(e.message.contains("`node_size`"), "{}", e.message);
        assert!(e.message.contains("`component_size`"), "{}", e.message);
    }
}

#[test]
fn component_size_old_name_is_an_unknown_key_to_get() {
    // req:config/component-size-setting@8f8238ab
    let s = Fixture::new(None, None).load(&[]).unwrap();
    let e = s.get("node_size").unwrap_err();
    assert!(
        e.message.contains("unknown key `node_size`"),
        "{}",
        e.message
    );
}

#[test]
fn component_size_errors_name_component_size() {
    // req:config/component-size-setting@8f8238ab
    for (text, needle) in [
        ("component_size = 5\n", "not 5"),
        ("[component_size]\ntokens = 5\n", "`tokens`"),
        ("[component_size]\nspec_tokens = 0\n", "spec_tokens"),
        ("[component_size]\ncode_tokens = \"10\"\n", "code_tokens"),
    ] {
        let f = Fixture::new(None, Some(text));
        let e = f.load(&[]).unwrap_err();
        assert_eq!(e.source, Some(f.repo_file().display().to_string()));
        assert!(e.message.contains("`component_size`"), "{}", e.message);
        assert!(e.message.contains(needle), "{}", e.message);
    }
}

#[test]
fn component_size_in_the_user_file_fails() {
    // req:config/component-size-setting@8f8238ab
    let e = error(
        &Fixture::new(Some("[component_size]\nspec_tokens = 1\n"), None),
        &[],
    );
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `component_size`"), "{e}");
}

#[test]
fn component_size_empty_table_is_from_the_file() {
    // req:config/component-size-setting@8f8238ab
    let f = Fixture::new(None, Some("[component_size]\n"));
    let s = f.load(&[]).unwrap();
    assert_eq!(s.node_size.value, NodeSize::default());
    assert_eq!(s.node_size.layer, Layer::File(f.repo_file()));
}

#[test]
fn component_size_has_no_variable() {
    // req:config/component-size-setting@8f8238ab
    let s = Fixture::new(None, None)
        .load(&[("CLM_COMPONENT_SIZE", "spec_tokens=1")])
        .unwrap();
    assert_eq!(s.node_size.value, NodeSize::default());
    assert_eq!(s.node_size.layer, Layer::Default);
}

#[test]
fn component_size_old_name_in_the_user_file_fails() {
    // req:config/component-size-setting@8f8238ab
    let e = error(
        &Fixture::new(Some("[node_size]\nspec_tokens = 1\n"), None),
        &[],
    );
    assert!(e.contains("config.toml"), "{e}");
    assert!(e.contains("unknown field `node_size`"), "{e}");
}

#[test]
fn upgrade_keys_are_set_in_their_own_layer() {
    // req:config/upgrade-settings@ec4b2181
    let f = Fixture::new(
        None,
        Some("upgraded = \"20261105-clm-upgrade\"\ndeclined_migrations = [\"n\", \"m\"]\n"),
    )
    .local("skip_migrations = [\"x\", \"y\"]\n");
    let s = f.load(&[]).unwrap();
    for (key, want, file) in [
        ("upgraded", "20261105-clm-upgrade", f.repo_file()),
        ("declined_migrations", "n,m", f.repo_file()),
        ("skip_migrations", "x,y", f.local_file()),
    ] {
        let (value, layer) = s.get(key).unwrap();
        assert_eq!(value, want, "{key}");
        assert_eq!(*layer, Layer::File(file), "{key}");
    }
}

#[test]
fn unset_upgrade_keys_print_as_keys_with_no_default() {
    // req:config/upgrade-settings@ec4b2181
    let s = Fixture::new(None, None).load(&[]).unwrap();
    for key in ["upgraded", "declined_migrations", "skip_migrations"] {
        let (value, layer) = s.get(key).unwrap();
        assert_eq!(value, "", "{key}");
        assert_eq!(*layer, Layer::Default, "{key}");
    }
}

#[test]
fn upgrade_keys_in_the_wrong_layer_or_type_fail_naming_file_and_key() {
    // req:config/upgrade-settings@ec4b2181
    // (user file, repository file, local file, file that fails, key)
    type Case = (
        Option<&'static str>,
        Option<&'static str>,
        Option<&'static str>,
        &'static str,
        &'static str,
    );
    let cases: [Case; 8] = [
        (
            None,
            Some("skip_migrations = [\"x\"]\n"),
            None,
            "clm.toml",
            "skip_migrations",
        ),
        (
            None,
            None,
            Some("upgraded = \"y\"\n"),
            "clm.local.toml",
            "upgraded",
        ),
        (
            None,
            None,
            Some("declined_migrations = [\"y\"]\n"),
            "clm.local.toml",
            "declined_migrations",
        ),
        (
            Some("upgraded = \"y\"\n"),
            None,
            None,
            "config.toml",
            "upgraded",
        ),
        (
            Some("skip_migrations = [\"y\"]\n"),
            None,
            None,
            "config.toml",
            "skip_migrations",
        ),
        (None, Some("upgraded = 3\n"), None, "clm.toml", "upgraded"),
        (
            None,
            Some("declined_migrations = [1]\n"),
            None,
            "clm.toml",
            "declined_migrations",
        ),
        (
            None,
            None,
            Some("skip_migrations = \"x\"\n"),
            "clm.local.toml",
            "skip_migrations",
        ),
    ];
    for (user, repo, local, file, key) in cases {
        let mut f = Fixture::new(user, repo);
        if let Some(local) = local {
            f = f.local(local);
        }
        let e = f.load(&[]).unwrap_err();
        assert!(e.to_string().contains(file), "{file} {key}: {e}");
        assert!(e.to_string().contains(key), "{file} {key}: {e}");
    }
}

#[test]
fn local_settings_path_is_the_path_loading_reads() {
    // req:config/local-settings-path@9aa79835
    let (_dir, main, linked) = worktrees();
    // The file doesn't exist, and the path is still the main worktree's.
    assert_eq!(
        clm_config::local_settings_path(&linked),
        Some(main.join("clm.local.toml"))
    );
    assert_eq!(
        clm_config::local_settings_path(&linked.join("tool")),
        Some(main.join("tool/clm.local.toml"))
    );
    fs::write(main.join("clm.local.toml"), "defects = \"off\"\n").unwrap();
    let s = no_user_load(&linked);
    assert_eq!(
        s.defects.layer,
        Layer::File(clm_config::local_settings_path(&linked).unwrap())
    );
}

#[test]
fn local_settings_path_of_a_bare_common_dir_is_none() {
    // req:config/local-settings-path@9aa79835
    let dir = TempDir::new().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let wt = base.join("repo.git/worktrees/w");
    fs::create_dir_all(&wt).unwrap();
    fs::write(wt.join("commondir"), "../..\n").unwrap();
    let root = base.join("w");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join(".git"), format!("gitdir: {}\n", wt.display())).unwrap();
    assert_eq!(clm_config::local_settings_path(&root), None);
}

#[test]
fn local_file_takes_skip_migrations_but_not_the_repository_state_keys() {
    // req:config/local-settings@6ef008e6
    let f = Fixture::new(None, None).local("skip_migrations = [\"x\"]\n");
    assert_eq!(f.load(&[]).unwrap().get("skip_migrations").unwrap().0, "x");
    for key in ["upgraded = \"y\"\n", "declined_migrations = [\"y\"]\n"] {
        let e = Fixture::new(None, None).local(key).load(&[]).unwrap_err();
        assert!(e.to_string().contains("clm.local.toml"), "{e}");
    }
}
