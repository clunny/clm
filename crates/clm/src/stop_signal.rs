//! `clm verify`'s stop by hand (`verify-command`): from its `verify passed ` line until
//! `clm-verify`'s call returns, the mutation pass and the filing of what it found together,
//! `SIGINT` and `SIGTERM` set the call's stop request instead of ending the process, and the
//! first of them decides the exit status once the call returns. At any other time they end it
//! as they would any process.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

/// The stop request `clm-verify`'s call watches.
pub static STOP: AtomicBool = AtomicBool::new(false);

/// The first signal caught while the handlers were installed, or 0.
static CAUGHT: AtomicI32 = AtomicI32::new(0);

const SIGNALS: [libc::c_int; 2] = [libc::SIGINT, libc::SIGTERM];

/// Async-signal-safe: two atomic operations. A further signal changes nothing.
extern "C" fn caught(signal: libc::c_int) {
    if CAUGHT
        .compare_exchange(0, signal, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        STOP.store(true, Ordering::SeqCst);
    }
}

/// The dispositions `install` replaced, which `restore` puts back.
pub struct Installed([libc::sigaction; 2]);

/// Catches `SIGINT` and `SIGTERM` until `restore`.
pub fn install() -> Installed {
    // SAFETY: `sigaction` with a zeroed struct, an empty mask and a handler that only touches
    // atomics.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = caught as extern "C" fn(libc::c_int) as libc::sighandler_t;
        action.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);
        let mut old: [libc::sigaction; 2] = std::mem::zeroed();
        for (signal, old) in SIGNALS.iter().zip(old.iter_mut()) {
            libc::sigaction(*signal, &action, old);
            // A signal the caller set to be ignored, as `nohup` or a background job does,
            // stays ignored.
            if old.sa_sigaction == libc::SIG_IGN {
                libc::sigaction(*signal, old, std::ptr::null_mut());
            }
        }
        Installed(old)
    }
}

impl Installed {
    /// Puts back the dispositions `install` found, and gives the first signal caught.
    pub fn restore(self) -> Option<libc::c_int> {
        for (signal, old) in SIGNALS.iter().zip(self.0.iter()) {
            // SAFETY: restores a disposition `sigaction` returned.
            unsafe {
                libc::sigaction(*signal, old, std::ptr::null_mut());
            }
        }
        match CAUGHT.load(Ordering::SeqCst) {
            0 => None,
            signal => Some(signal),
        }
    }
}
