use crate::{ShellError, Span};
use nu_glob::Interruptible;
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub trait Signal: Send + Sync {
    fn set(&self, value: bool);
    fn get(&self) -> bool;
}

/// Used to check for signals to suspend or terminate the execution of Nushell code.
///
/// For now, this struct only supports interruption (ctrl+c or SIGINT).
#[derive(Clone)]
pub struct Signals {
    signals: Option<Arc<dyn Signal>>,
}

impl std::fmt::Debug for Signals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signals")
            .field("signals", &self.signals.as_ref().map(|s| s.get()))
            .finish()
    }
}

impl Signals {
    /// A [`Signals`] that is not hooked up to any event/signals source.
    ///
    /// So, this [`Signals`] will never be interrupted.
    pub const EMPTY: Self = Signals { signals: None };

    /// Create a new [`Signals`] with `ctrlc` as the interrupt source.
    ///
    /// Once `ctrlc` is set to `true`, [`check`](Self::check) will error
    /// and [`interrupted`](Self::interrupted) will return `true`.
    pub fn new(ctrlc: Arc<dyn Signal>) -> Self {
        Self {
            signals: Some(ctrlc),
        }
    }

    /// Create a [`Signals`] that is not hooked up to any event/signals source.
    ///
    /// So, the returned [`Signals`] will never be interrupted.
    ///
    /// This should only be used in test code, or if the stream/iterator being created
    /// already has an underlying [`Signals`].
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Returns an `Err` if an interrupt has been triggered.
    ///
    /// Otherwise, returns `Ok`.
    #[inline]
    pub fn check(&self, span: &Span) -> Result<(), ShellError> {
        #[inline]
        #[cold]
        fn interrupt_error(span: &Span) -> Result<(), ShellError> {
            Err(ShellError::Interrupted { span: *span })
        }

        if self.interrupted() {
            interrupt_error(span)
        } else {
            Ok(())
        }
    }

    /// Triggers an interrupt.
    pub fn trigger(&self) {
        if let Some(signals) = &self.signals {
            signals.set(true);
        }
    }

    /// Returns whether an interrupt has been triggered.
    #[inline]
    pub fn interrupted(&self) -> bool {
        self.signals
            .as_deref()
            .is_some_and(|b| b.get())
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.signals.is_none()
    }

    pub fn reset(&self) {
        if let Some(signals) = &self.signals {
            signals.set(false);
        }
    }
}

impl Signal for AtomicBool {
    #[inline]
    fn set(&self, value: bool) {
        self.store(value, Ordering::Relaxed);
    }

    #[inline]
    fn get(&self) -> bool {
        self.load(Ordering::Relaxed)
    }
}

impl Interruptible for Signals {
    #[inline]
    fn interrupted(&self) -> bool {
        self.interrupted()
    }
}

/// The types of things that can be signaled. It's anticipated this will change as we learn more
/// about how we'd like signals to be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalAction {
    Interrupt,
    Reset,
}
