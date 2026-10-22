//! A spinner on stderr for a run that takes more than a few seconds: hidden until then, so a quick run prints
//! nothing extra, and never drawn when stderr is not a terminal.

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

/// How long a run goes before the spinner shows
const AFTER: Duration = Duration::from_secs(2);

pub struct Delayed {
    bar: ProgressBar,
    done: mpsc::Sender<()>,
    shower: JoinHandle<()>,
}

impl Delayed {
    pub fn start() -> Delayed {
        let bar = ProgressBar::with_draw_target(None, ProgressDrawTarget::hidden()).with_style(
            ProgressStyle::with_template("{spinner} {msg} {elapsed}").expect("a valid template"),
        );
        let (done, wait) = mpsc::channel::<()>();
        let b = bar.clone();
        let shower = std::thread::spawn(move || {
            if wait.recv_timeout(AFTER) == Err(mpsc::RecvTimeoutError::Timeout) {
                // stderr() draws nothing when stderr is not a terminal
                b.set_draw_target(ProgressDrawTarget::stderr());
                b.enable_steady_tick(Duration::from_millis(100));
            }
        });
        Delayed { bar, done, shower }
    }

    /// What the run is doing now
    pub fn set(&self, msg: String) {
        self.bar.set_message(msg);
    }

    /// Clears the spinner, before the output is written
    pub fn finish(self) {
        let _ = self.done.send(());
        let _ = self.shower.join();
        self.bar.finish_and_clear();
    }
}
