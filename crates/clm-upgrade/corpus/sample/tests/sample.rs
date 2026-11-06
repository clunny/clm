// @REQ@:sample/greets@77af7002
#[test]
fn greets_by_name() {
    assert_eq!(format!("Hello, {}", "Ada"), "Hello, Ada");
}
