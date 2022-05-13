//! Simple graceful shutdown manager for async and concurrent code.
//!
//! Provides abstractions to easily propagate and listen/wait for shutdown signals
//! to all components of a system.
//! 
//! The [Manager] is the coordinator, usually owned by `main()`, that coordinates
//! the shutdown by sending termination signals to each subscribed component and
//! waits for their termination.
//! 
//! Every component that needs to listen for the shutdown event should receive
//! ownership of a instance of [Shutdown], produced either via [Manager::subscribe()]
//! or by cloning of an existing instance, and use the async [Shutdown::recv()] method which
//! returns as soon as the shutdown signal has been issued by the manager.
//! 
//! The component may also query [Shutdown::is_shutdown()] to check whether the shutdown
//! signal has already been received.
//! 
//! This module derives from the [Tokio.rs documentation](https://tokio.rs/tokio/topics/shutdown)
//! for graceful shutdown, which made use broadcast channels to
//! propagate signals. This implementation uses watch channels instead, and the
//! original example has been further extended with cloning of `Shutdown` instances
//! and with the concept of [Manager], which are aimed at making even easier the usage
//! of this module.
//! 
//! # Examples
//! 
//! ```
//! use tokio;
//! use std::{thread, time};
//! use hass::sync::shutdown::{Shutdown, Manager};
//!
//! async fn worker(shutdown: Shutdown) {
//!     let mut shutdown = shutdown;
//!     println!("worker's busy");
//!     // do stuff
//!     shutdown.recv().await;
//!     println!("worker exited");
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let manager = Manager::new();
//!     let shutdown = manager.subscribe();
//!     tokio::spawn(worker(shutdown));
//!     thread::sleep(time::Duration::from_secs(3));
//!     manager.shutdown().await;
//!     println!("main exited");
//! }
//! ```


use tokio::sync::{mpsc, watch};

/// Receives and remembers shutdown signals
/// 
/// Instances of `Shutdown` may be used to either wait for the shutdown
/// signal from their respective [Manager] via [Shutdown::recv()], or
/// check whether a previous call to `recv()` already received it.
/// 
/// New instances may be created either via [Manager::subscribe()],
/// or via [Shutdown::clone()]. The former is more common for top-level
/// components, while the latter is the way to go when a component needs
/// to spawn another but has no direct access to the [Manager].
#[derive(Clone,Debug)]
pub struct Shutdown {
    shutdown: bool,
    notify: watch::Receiver<()>,
    _shutdown_complete: mpsc::Sender<()>,
}

impl Shutdown {

    /// Returns `true` if the shutdown signal has already been received by
    /// [Shutdown::recv()].
    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    /// Receive the shutdown notice, waiting if necessary.
    /// 
    /// The method waits for the shutdown signal from the [Manager], and may be
    /// therefore used to coordinate the shutdown of the component that owns it.
    /// 
    /// Upon returning once, subsequent calls to the method will return immediately
    /// without waiting.
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

/// Manages [Shutdown] instances, coordinates signals, and waits for termination.
/// 
/// The roles of a shutdown `Manager` are (1) to keep coordination for graceful
/// shutdown with all components that subscribed to it and thus received a
/// [Shutdown] instance, and (2) let the owner of the manager request and wait
/// at the same time for the shutdown to complete.
pub struct Manager {
    notify_shutdown: watch::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    shutdown_complete_tx: mpsc::Sender<()>,
}

impl Manager {

    /// Creates a new manager.
    pub fn new() -> Manager {
        let (notify_shutdown, _) = watch::channel(());
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);

        Manager {
            notify_shutdown,
            shutdown_complete_rx,
            shutdown_complete_tx
        }
    }

    /// Returns a [Shutdown] object subscribed to the manager.
    pub fn subscribe(&self) -> Shutdown {
        Shutdown {
            shutdown: false,
            notify: self.notify_shutdown.subscribe(),
            _shutdown_complete: self.shutdown_complete_tx.clone(),
        }
    }

    /// Consumes the manager and waits for all [Shutdown] subscribed instances
    /// to terminate. Subscribed instances include both those created via 
    /// [Manager::subscribe()] and [Shutdown::clone()].
    pub async fn shutdown(mut self) {
        drop(self.notify_shutdown);
        drop(self.shutdown_complete_tx);
        let _ = self.shutdown_complete_rx.recv().await;
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}