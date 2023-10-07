use std::sync::mpsc::{channel, Receiver, Sender};
use dcommon::concurrent::spawn_thread_with_name;
use tracing::error;

/// Creates a `Sender`, `Receiver` pair that can be used to load data in a background thread.
///
/// The output can be received on the `Receiver` if `send_output` is `true`.
///
/// Drop the sender to stop the thread.
///
/// # Errors
///
/// Will return `Err` if the thread cannot be spawned.
pub fn thread_loader<I, F, O>(send_output: bool, mut f: F) -> (Sender<I>, Receiver<O>)
    where
        I: Send + 'static,
        F: FnMut(I) -> O + Send + 'static,
        O: Send + 'static,
{
    let (tx_in, rx_in) = channel::<I>();
    let (tx_out, rx_out) = channel::<O>();
    spawn_thread_with_name("loader", move || {
        while let Ok(input) = rx_in.recv() {
            let o = f(input);
            if send_output {
                if let Err(e) = tx_out.send(o) {
                    error!(error = %e, "Failed to send output");
                }
            }
        }
    });
    (tx_in, rx_out)
}
