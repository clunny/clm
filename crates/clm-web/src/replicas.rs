//! The repo's `Replica` objects (`web.worker.replica`), reached through the `REPLICA`
//! binding. The Worker is their only caller; tests fake this trait.

/// One `Replica` Durable Object per repo, named `<owner>/<repo>`.
#[allow(async_fn_in_trait)]
pub trait Replicas {
    /// Sends `method path` (the path may carry a query), with `body` for a POST, to the
    /// object named `name`: its status and body, or `None` without the binding or when the
    /// request fails.
    async fn replica(
        &self,
        name: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
    ) -> Option<(u16, String)>;
}
