//! A concurrent multi-producer multi-consumer queue.
//!
//! There are two kinds of queues:
//!
//! 1. [Bounded] queue with limited capacity.
//! 2. [Unbounded] queue with unlimited capacity.
//!
//! Queues also have the capability to get [closed] at any point. When closed, no more items can be
//! pushed into the queue, although the remaining items can still be popped.
//!
//! These features make it easy to build channels similar to [`std::sync::mpsc`] on top of this
//! crate.
//!
//! # Examples
//!
//! ```
//! use concurrent_queue::ConcurrentQueue;
//!
//! let q = ConcurrentQueue::unbounded();
//! q.push(1).unwrap();
//! q.push(2).unwrap();
//!
//! assert_eq!(q.pop(), Ok(1));
//! assert_eq!(q.pop(), Ok(2));
//! ```
//!
//! [Bounded]: `ConcurrentQueue::bounded()`
//! [Unbounded]: `ConcurrentQueue::unbounded()`
//! [closed]: `ConcurrentQueue::close()`

#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

use std::error;
use std::fmt;

use crate::bounded::Bounded;
use crate::unbounded::Unbounded;

mod bounded;
mod unbounded;

/// A concurrent queue.
///
/// # Examples
///
/// ```
/// use concurrent_queue::{ConcurrentQueue, PopError, PushError};
///
/// let q = ConcurrentQueue::bounded(2);
///
/// assert_eq!(q.push('a'), Ok(()));
/// assert_eq!(q.push('b'), Ok(()));
/// assert_eq!(q.push('c'), Err(PushError::Full('c')));
///
/// assert_eq!(q.pop(), Ok('a'));
/// assert_eq!(q.pop(), Ok('b'));
/// assert_eq!(q.pop(), Err(PopError::Empty));
/// ```
pub struct ConcurrentQueue<T>(Inner<T>);

unsafe impl<T: Send> Send for ConcurrentQueue<T> {}
unsafe impl<T: Send> Sync for ConcurrentQueue<T> {}

enum Inner<T> {
    Bounded(Bounded<T>),
    Unbounded(Unbounded<T>),
}

impl<T> ConcurrentQueue<T> {
    /// Creates a new bounded queue.
    ///
    /// The queue allocates enough space for `cap` items.
    ///
    /// # Panics
    ///
    /// If the capacity is zero, this constructor will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::ConcurrentQueue;
    ///
    /// let q = ConcurrentQueue::<i32>::bounded(100);
    /// ```
    pub fn bounded(cap: usize) -> ConcurrentQueue<T> {
        ConcurrentQueue(Inner::Bounded(Bounded::new(cap)))
    }

    /// Creates a new unbounded queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::ConcurrentQueue;
    ///
    /// let q = ConcurrentQueue::<i32>::unbounded();
    /// ```
    pub fn unbounded() -> ConcurrentQueue<T> {
        ConcurrentQueue(Inner::Unbounded(Unbounded::new()))
    }

    /// Attempts to push an item into the queue.
    ///
    /// If the queue is full or closed, the item is returned back as an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::{ConcurrentQueue, PushError};
    ///
    /// let q = ConcurrentQueue::bounded(1);
    ///
    /// // Push succeeds because there is space in the queue.
    /// assert_eq!(q.push(10), Ok(()));
    ///
    /// // Push errors because the queue is now full.
    /// assert_eq!(q.push(20), Err(PushError::Full(20)));
    ///
    /// // Close the queue, which will prevent further pushes.
    /// q.close();
    ///
    /// // Pushing now errors indicating the queue is closed.
    /// assert_eq!(q.push(20), Err(PushError::Closed(20)));
    ///
    /// // Pop the single item in the queue.
    /// assert_eq!(q.pop(), Ok(10));
    ///
    /// // Even though there is space, no more items can be pushed.
    /// assert_eq!(q.push(20), Err(PushError::Closed(20)));
    /// ```
    pub fn push(&self, value: T) -> Result<(), PushError<T>> {
        match &self.0 {
            Inner::Bounded(q) => q.push(value),
            Inner::Unbounded(q) => q.push(value),
        }
    }

    /// Attempts to pop an item from the queue.
    ///
    /// If the queue is empty, an error is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::{ConcurrentQueue, PopError};
    ///
    /// let q = ConcurrentQueue::bounded(1);
    ///
    /// // Pop errors when the queue is empty.
    /// assert_eq!(q.pop(), Err(PopError::Empty));
    ///
    /// // Push one item and close the queue.
    /// assert_eq!(q.push(10), Ok(()));
    /// q.close();
    ///
    /// // Remaining items can be popped.
    /// assert_eq!(q.pop(), Ok(10));
    ///
    /// // Again, pop errors when the queue is empty,
    /// // but now also indicates that the queue is closed.
    /// assert_eq!(q.pop(), Err(PopError::Closed));
    /// ```
    pub fn pop(&self) -> Result<T, PopError> {
        match &self.0 {
            Inner::Bounded(q) => q.pop(),
            Inner::Unbounded(q) => q.pop(),
        }
    }

    /// Returns `true` if the queue is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::ConcurrentQueue;
    ///
    /// let q = ConcurrentQueue::<i32>::unbounded();
    ///
    /// assert!(q.is_empty());
    /// q.push(1).unwrap();
    /// assert!(!q.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            Inner::Bounded(q) => q.is_empty(),
            Inner::Unbounded(q) => q.is_empty(),
        }
    }

    /// Returns `true` if the queue is full.
    ///
    /// An unbounded queue is never full.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::ConcurrentQueue;
    ///
    /// let q = ConcurrentQueue::bounded(1);
    ///
    /// assert!(!q.is_full());
    /// q.push(1).unwrap();
    /// assert!(q.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        match &self.0 {
            Inner::Bounded(q) => q.is_full(),
            Inner::Unbounded(q) => q.is_full(),
        }
    }

    /// Returns the number of items in the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::ConcurrentQueue;
    ///
    /// let q = ConcurrentQueue::unbounded();
    /// assert_eq!(q.len(), 0);
    ///
    /// assert_eq!(q.push(10), Ok(()));
    /// assert_eq!(q.len(), 1);
    ///
    /// assert_eq!(q.push(20), Ok(()));
    /// assert_eq!(q.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        match &self.0 {
            Inner::Bounded(q) => q.len(),
            Inner::Unbounded(q) => q.len(),
        }
    }

    /// Returns the capacity of the queue.
    ///
    /// Unbounded queues have infinite capacity, represented as [`None`].
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::ConcurrentQueue;
    ///
    /// let q = ConcurrentQueue::<i32>::bounded(7);
    /// assert_eq!(q.capacity(), Some(7));
    ///
    /// let q = ConcurrentQueue::<i32>::unbounded();
    /// assert_eq!(q.capacity(), None);
    /// ```
    pub fn capacity(&self) -> Option<usize> {
        match &self.0 {
            Inner::Bounded(q) => Some(q.capacity()),
            Inner::Unbounded(_) => None,
        }
    }

    /// Closes the queue.
    ///
    /// Returns `true` if this call closed the queue, or `false` if it was already closed.
    ///
    /// When a queue is closed, no more items can be pushed but the remaining items can still be
    /// popped.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::{ConcurrentQueue, PopError, PushError};
    ///
    /// let q = ConcurrentQueue::unbounded();
    /// assert_eq!(q.push(10), Ok(()));
    ///
    /// assert!(q.close());  // `true` because this call closes the queue.
    /// assert!(!q.close()); // `false` because the queue is already closed.
    ///
    /// // Cannot push any more items when closed.
    /// assert_eq!(q.push(20), Err(PushError::Closed(20)));
    ///
    /// // Remaining items can still be popped.
    /// assert_eq!(q.pop(), Ok(10));
    ///
    /// // When no more items are present, the error is `Closed`.
    /// assert_eq!(q.pop(), Err(PopError::Closed));
    /// ```
    pub fn close(&self) -> bool {
        match &self.0 {
            Inner::Bounded(q) => q.close(),
            Inner::Unbounded(q) => q.close(),
        }
    }

    /// Returns `true` if the queue is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// use concurrent_queue::ConcurrentQueue;
    ///
    /// let q = ConcurrentQueue::<i32>::unbounded();
    ///
    /// assert!(!q.is_closed());
    /// q.close();
    /// assert!(q.is_closed());
    /// ```
    pub fn is_closed(&self) -> bool {
        match &self.0 {
            Inner::Bounded(q) => q.is_closed(),
            Inner::Unbounded(q) => q.is_closed(),
        }
    }
}

impl<T> fmt::Debug for ConcurrentQueue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConcurrentQueue")
            .field("len", &self.len())
            .field("capacity", &self.capacity())
            .field("is_closed", &self.is_closed())
            .finish()
    }
}

/// Error which occurs when popping from an empty queue.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PopError {
    /// The queue is empty but not closed.
    Empty,

    /// The queue is empty and closed.
    Closed,
}

impl error::Error for PopError {}

impl fmt::Debug for PopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::Empty => write!(f, "Empty"),
            PopError::Closed => write!(f, "Closed"),
        }
    }
}

impl fmt::Display for PopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PopError::Empty => write!(f, "Empty"),
            PopError::Closed => write!(f, "Closed"),
        }
    }
}

/// Error which occurs when pushing into a full or closed queue.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum PushError<T> {
    /// The queue is full but not closed.
    Full(T),

    /// The queue is closed.
    Closed(T),
}

impl<T: fmt::Debug> error::Error for PushError<T> {}

impl<T: fmt::Debug> fmt::Debug for PushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PushError::Full(t) => f.debug_tuple("Full").field(t).finish(),
            PushError::Closed(t) => f.debug_tuple("Closed").field(t).finish(),
        }
    }
}

impl<T> fmt::Display for PushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PushError::Full(_) => write!(f, "Full"),
            PushError::Closed(_) => write!(f, "Closed"),
        }
    }
}
