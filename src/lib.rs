#[cfg(feature = "async")]
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("system error or I/O failure")]
    IoFailure(#[from] std::io::Error),

    #[error("the implementation returns malformed strings")]
    InvalidString(#[from] std::string::FromUtf8Error),

    #[error("failed to parse the string returned from implementation")]
    UnexpectedOutput(&'static str),

    #[error("cannot find any dialog implementation (kdialog/zanity)")]
    NoImplementation,

    #[error("the implementation reports error")]
    ImplementationError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Dialog {
    type Output;

    fn show(self) -> Result<Self::Output>;

    /// Create an asynchronous dialog
    #[cfg(feature = "async")]
    fn create_async(self) -> AsyncDialog<Result<Self::Output>>;
}

/// An asynchronous dialog.
/// Can be used manually or as a `Future`.
///
/// Does nothing until it's either polled once as a `Future` or `show_dialog` is called.
/// Once this has been done, a background thread will be created which is responsible for
/// sending a result back. If `AsyncDialog` is used as a `Future`, the background thread
/// will also be responsible for waking the `AsyncDialog`.
#[cfg(feature = "async")]
pub struct AsyncDialog<T> {
    spawn: RefCell<Option<Box<dyn FnOnce(Option<std::task::Waker>) + Send + Sync + 'static>>>,
    // We have to use `crossbeam_channel` instead of `std::sync:mpsc` because `Receiver` needs to be `Send` and `Sync`.
    output: crossbeam_channel::Receiver<T>,
    is_future: Cell<bool>,
}

#[cfg(feature = "async")]
impl<T> AsyncDialog<T> {
    pub(crate) fn new<F>(spawn: F, output: crossbeam_channel::Receiver<T>) -> Self
    where
        F: FnOnce(Option<std::task::Waker>) + Send + Sync + 'static,
    {
        Self {
            spawn: RefCell::new(Some(Box::new(spawn))),
            output,
            is_future: Cell::new(true),
        }
    }

    /// Try retrieving the result of the dialog. Does not block. Returns `TryRecvError::Empty` if
    /// there is nothing to retrieve.
    pub fn try_recv(&self) -> std::result::Result<T, TryRecvError> {
        self.output.try_recv().map_err(|err| match err {
            crossbeam_channel::TryRecvError::Empty => TryRecvError::Empty,
            crossbeam_channel::TryRecvError::Disconnected => {
                TryRecvError::Disconnected(Disconnected)
            }
        })
    }

    /// Block the current thread while waiting for the dialog's result.
    pub fn recv(&self) -> std::result::Result<T, Disconnected> {
        self.output.recv().map_err(|_| Disconnected)
    }

    /// Show the dialog by spawning a background thread.
    ///
    /// Once this function has been called, the `AsyncDialog` can no longer be used as a `Future`.
    /// If the `AsyncDialog` is polled furter, it will remain `Pending` indefinitely.
    pub fn show_dialog(&self) {
        self._show_dialog(None);
        self.is_future.set(false);
    }

    fn _show_dialog(&self, waker: Option<&std::task::Waker>) -> bool {
        if let Some(spawn) = self.spawn.borrow_mut().take() {
            spawn(waker.map(|waker| waker.clone()));
            true
        } else {
            false
        }
    }
}

#[cfg(feature = "async")]
impl<T> Future for AsyncDialog<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.is_future.get() {
            return Poll::Pending;
        }

        if self._show_dialog(Some(cx.waker())) {
            Poll::Pending
        } else {
            // TODO: Is unwrapping here acceptable?
            Poll::Ready(self.output.recv().unwrap())
        }
    }
}

#[cfg(feature = "async")]
#[derive(Error, Debug)]
pub enum TryRecvError {
    #[error("The channel is currently empty")]
    Empty,
    #[error("{0}")]
    Disconnected(#[from] Disconnected),
}

#[cfg(feature = "async")]
#[derive(Error, Debug)]
#[error("The internal channel's sending half has been disconnected.")]
pub struct Disconnected;

mod message;
pub use message::*;

mod file;
pub use file::*;

mod r#impl;
