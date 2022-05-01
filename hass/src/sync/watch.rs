use tokio::sync::watch;

/// Waits for the server shutdown signal.
/// Shutdown is signalled by the app by using a `watch::Receiver`
/// on which a single value is ever sent to notify it.
/// 
/// This is a variant of https://tokio.rs/tokio/topics/shutdown which
/// uses broadcast channels instead, as suggested by the tutorial itself.
#[derive(Debug)]
pub struct Shutdown {
    shutdown: bool,
    notify: watch::Receiver<()>,
}

impl Shutdown {
    /// Create a new `Shutdown` backed by the given `broadcast::Receiver`.
    pub fn new(notify: watch::Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify,
        }
    }

    /// Returns `true` if the shutdown signal has already been received.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    pub async fn recv(&mut self) {
        if self.shutdown {
            return;
        }
        // Should the sender be dropped or not, we interptet it as
        // a shutdown anyway.
        let _ = self.notify.changed().await;
        self.shutdown = true;
    }
}