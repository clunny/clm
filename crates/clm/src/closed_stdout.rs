//! A reader of stdout that has gone ends the command quietly (`clm/closed-stdout`).
//!
//! The Rust runtime ignores `SIGPIPE`, so a print to a closed pipe fails and `println!`
//! panics. `run` silences that one panic and turns it into exit status 141.

use std::io::ErrorKind;
use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;

/// The status a shell reports for a tool `SIGPIPE` killed; no command's own exit codes use it.
const SIGPIPE_STATUS: u8 = 141;

const PREFIX: &str = "failed printing to stdout: ";

/// Whether a panic message is std's report of a print to a stdout whose reader has gone.
fn is_closed_stdout(message: &str) -> bool {
    let Some(reason) = message.strip_prefix(PREFIX) else {
        return false;
    };
    let Some(code) = reason
        .strip_suffix(')')
        .and_then(|r| r.rsplit_once("(os error "))
        .and_then(|(_, n)| n.parse::<i32>().ok())
    else {
        return false;
    };
    std::io::Error::from_raw_os_error(code).kind() == ErrorKind::BrokenPipe
}

/// Runs `body` with the panic hook installed; a closed-stdout panic becomes exit 141 and
/// any other panic keeps unwinding.
pub fn run(body: impl FnOnce() -> ExitCode) -> ExitCode {
    let default = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if !info.payload_as_str().is_some_and(is_closed_stdout) {
            default(info);
        }
    }));
    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(code) => code,
        Err(payload) => {
            let closed = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .is_some_and(is_closed_stdout);
            if closed {
                ExitCode::from(SIGPIPE_STATUS)
            } else {
                panic::resume_unwind(payload)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_broken_pipe_counts() {
        // req:clm/closed-stdout@e1df9220
        assert!(is_closed_stdout(
            "failed printing to stdout: Broken pipe (os error 32)"
        ));
        assert!(!is_closed_stdout(
            "failed printing to stdout: No space left on device (os error 28)"
        ));
        assert!(!is_closed_stdout("Broken pipe (os error 32)"));
        assert!(!is_closed_stdout("failed printing to stdout: Broken pipe"));
    }
}
