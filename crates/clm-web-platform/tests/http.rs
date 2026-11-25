//! Percent coding, and a query parameter's value (`percent-coding`).

use clm_web_platform::http::{Request, percent_decode, percent_encode};

#[test]
fn percent_coding() {
    // req:web.worker.platform/percent-coding@d15576c0
    assert_eq!(percent_encode("a b/é"), "a%20b%2F%C3%A9");
    assert_eq!(percent_encode("Az09-_.~"), "Az09-_.~");
    for kept in ["100%", "%2", "%zz"] {
        assert_eq!(percent_decode(kept), kept);
    }
    assert_eq!(percent_decode("%ff"), "\u{FFFD}");
    assert_eq!(percent_decode("spec/a+b"), "spec/a+b");
    assert_eq!(percent_decode("spec%2Fa%2Bb"), "spec/a+b");
    let req = Request {
        query: "q=a+b%2B&q=c&r".into(),
        ..Request::default()
    };
    assert_eq!(req.param("q").as_deref(), Some("a b+"));
    assert_eq!(req.param("r"), None);
}
