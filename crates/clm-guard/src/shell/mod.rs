//! How the guard reads a Bash command as the shell would (`plugin.hooks.guard.shell`): where it
//! splits into segments and tokens, which token is a segment's command, and the pieces to judge,
//! each with its checkout.

pub(crate) mod command;
mod scan;
pub(crate) mod walk;
