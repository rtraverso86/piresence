use tokio::sync::{mpsc, watch};

/// Waits for the server shutdown signal.
/// Shutdown is signalled by the app by using a `watch::Receiver`
/// on which a single value is ever sent to notify it.
/// 
/// This is a variant of https://tokio.rs/tokio/topics/shutdown which
/// uses broadcast channels instead, as suggested by the tutorial itself.
#[derive(Clone,Debug)]
pub struct Shutdown {
    shutdown: bool,
    notify: watch::Receiver<()>,
    _shutdown_complete: mpsc::Sender<()>,
}

impl Shutdown {

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

pub struct Manager {
    notify_shutdown: watch::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Manager {

    pub fn new() -> Manager {
        let (notify_shutdown, _) = watch::channel(());
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);
    
        Manager {
            notify_shutdown,
            shutdown_complete_rx,
            shutdown_complete_tx
        }
    }

    pub fn subscribe(&self) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify: self.notify_shutdown.subscribe(),
            _shutdown_complete: self.shutdown_complete_tx.clone(),
        }
    }

    pub async fn shutdown(mut self) {
        drop(self.notify_shutdown);
        drop(self.shutdown_complete_tx);
        let _ = self.shutdown_complete_rx.recv().await;
    }
}
