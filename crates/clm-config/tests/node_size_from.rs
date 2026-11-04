use clm_config::{NodeSize, node_size_from};

#[test]
fn partial_table() {
    // req:config/component-size-from-text@37254aae
    assert_eq!(
        node_size_from("[component_size]\ncode_tokens = 60000\n").unwrap(),
        NodeSize {
            spec_tokens: 12000,
            code_tokens: 60000
        }
    );
}

#[test]
fn no_table() {
    // req:config/component-size-from-text@37254aae
    let size = node_size_from("[land]\ncheck = \"scripts/clm-check.sh\"\n").unwrap();
    assert_eq!(size, NodeSize::default());
    assert_eq!(
        (size.spec_tokens, size.code_tokens),
        (12000, 40000),
        "the defaults"
    );
    assert_eq!(node_size_from("").unwrap(), NodeSize::default());
}

#[test]
fn invalid_value() {
    // req:config/component-size-from-text@37254aae
    let e = node_size_from("[component_size]\nspec_tokens = 0\n").unwrap_err();
    assert_eq!(e.source.as_deref(), Some("clm.toml"));
    assert!(e.message.contains("`component_size`"), "{}", e.message);
}

#[test]
fn fails_wherever_loading_the_repository_file_does() {
    // req:config/component-size-from-text@37254aae
    let valid = "[component_size]\ncode_tokens = 60000\n";
    for (extra, names) in [
        ("colour = \"red\"\n", "colour"),
        ("writer = \"christopher\"\n", "writer"),
        ("merge = \"push\"\n", "merge"),
        ("host = \"gitlab\"\n", "host"),
        ("max_builds = 0\n", "max_builds"),
        ("[land]\ncheck = \"\"\n", "land"),
        ("[[detectors]]\nname = \"m\"\ncommand = []\n", "detectors"),
        (
            "[[detectors]]\nname = \"m\"\ncommand = [\"m\"]\ntimeout = 0\n",
            "detectors",
        ),
        ("[affected]\nall = [\"\"]\n", "affected"),
        ("not toml", "TOML parse error"),
    ] {
        let text = format!("{valid}{extra}");
        let e = node_size_from(&text).unwrap_err();
        assert_eq!(e.source.as_deref(), Some("clm.toml"), "{text}");
        assert!(e.message.contains(names), "{text}: {}", e.message);
    }
}

#[test]
fn component_size_from_text() {
    // req:config/component-size-setting@8f8238ab
    assert_eq!(
        node_size_from("[component_size]\ncode_tokens = 60000\n").unwrap(),
        NodeSize {
            spec_tokens: 12000,
            code_tokens: 60000
        }
    );
    let e = node_size_from("[node_size]\nspec_tokens = 1\n").unwrap_err();
    assert_eq!(e.source.as_deref(), Some("clm.toml"));
    assert!(e.message.contains("`component_size`") && e.message.contains("`node_size`"));
}
