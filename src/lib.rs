//! Thread-safe clone-on-write container with lock-less reading
//!
//! The SyncCow is a container for concurrent writing and reading of data. It's intended to be a
//! faster alternative to `std::sync::RwLock`. Especially scenarios with many concurrent readers
//! heavily benefit from the SyncCow. Reading is guaranteed to
//! be lock-less and return immediately. Writing is only blocked by other write-accesses, never by
//! any read-access. A SyncCow with only one writer and arbitrary readers will never block. 
//! As SyncCow stores two copies of it's contained value and read values are handed out as
//! std::sync::Arc, a program using SyncCow might have a higher memory-footprint compared to
//! std::sync::RwLock.
//!
//! Note that readers might read outdated data when using the SyncCow,
//! as writing and reading concurrently is possible.
//! If that is indesireable consider std::sync::RwLock.
//!
//! Usage is similar to RwLock, but writing to the container is done through edit and a closure
//! instead of acquiring a write-lock:
//! ```
#![doc = include_str!("../examples/write_and_read_thread.rs")]
//! ```

use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicPtr, AtomicUsize};
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

/// Thread-safe clone-on-write container with lock-less reading. 
///
/// See crate documentation for a full code example
pub struct SyncCow<T: Clone> {
    write_lock: Mutex<()>,
    latest: AtomicUsize,
    atomic_red: (AtomicPtr<Arc<T>>, AtomicUsize),
    atomic_green: (AtomicPtr<Arc<T>>, AtomicUsize),
}

const RED: usize = 0;
const GREEN: usize = 1;

impl<T: Clone> SyncCow<T> {
    /// Edit the contents of the SyncCow. Blocks to acquire write-lock.
    ///
    /// The edit function will block until the current writer is done and the write-lock could be
    /// acquired. Once the lock has been acquired, the contained object is cloned, and `edit_fn` is
    /// called with the cloned object as argument. After the `edit_fn` has returned, the write-lock
    /// is released and the internal object pointer is updated so readers read the cloned-and-edited object.
    ///
    /// ```
    /// let cow = sync_cow::SyncCow::new(5);
    /// cow.edit(|x| *x = 6);
    /// assert_eq!(*cow.read(), 6);
    /// ```
    pub fn edit<F>(&self, edit_fn: F)
    where
        F: FnOnce(&mut T),
    {
        // The write-lock prevents multiple concurrent writers, but does not inhibit readers
        let _lck = self.write_lock.lock().unwrap();
        let latest = self.latest.load(Relaxed);

        // We need to clone latest, but update the older pointer.
        let ((old_ptr, old_cnt), latest_ptr) = match latest {
            RED => (&self.atomic_green, &self.atomic_red.0),
            GREEN => (&self.atomic_red, &self.atomic_green.0),
            _ => panic!("Latest does not exist. This should never happen."),
        };

        // Clone latest
        let load_ptr = latest_ptr.load(Relaxed);
        let obj = unsafe { &*load_ptr };
        let mut cloned = Box::new(Arc::new(obj.as_ref().clone()));

        // And let the user-provided callback edit it
        edit_fn(&mut Arc::get_mut(cloned.as_mut()).unwrap());

        // This releases the pointer of the Arc from the Box, such that it is not automatically freed
        let new_ptr = Box::into_raw(cloned);

        // Override the old ptr, let the previous "latest_ptr" still be read by late readers
        let old_ptr = old_ptr.swap(new_ptr, Relaxed);

        // And wait until any late readers still reading the older ptr finished cloning the Arc
        while old_cnt.load(Relaxed) != 0 {
            std::thread::yield_now();
        }

        // Now guide all readers to the newly updated Arc
        self.latest.store((latest + 1) % 2, Relaxed);

        // Ensures Arc pointed to by old_ptr will be released at return
        let _ = unsafe { Box::from_raw(old_ptr) };
    }

    /// Get the current value of the SyncCow as immutable std::sync::Arc.
    ///
    /// The `read` function will return the latest version of the SyncCow's value as an Arc.
    /// This Arc contains an immutable state of the SyncCow's value; call `read()` again to obtain
    /// an updated state after a writer has edited the value.
    /// The reader can decide when to drop the Arc; the value will be dropped when a writer has
    /// updated the value and no reader keeps an Arc of this value-state alive.
    ///
    /// ```
    /// let cow = sync_cow::SyncCow::new(5);
    /// let val = cow.read();
    /// assert_eq!(*val, 5);
    /// cow.edit(|x| *x = 6);
    /// assert_eq!(*val, 5);  // Arc keeps old value
    /// assert_eq!(*cow.read(), 6); // Another read returns new value
    /// ```
    pub fn read(&self) -> Arc<T> {
        let latest = self.latest.load(Relaxed);
        // We want to read whatever has been updated last
        let (ptr, cnt) = match latest {
            RED => &self.atomic_red,
            GREEN => &self.atomic_green,
            _ => panic!("Latest does not exist. This should never happen."),
        };

        // Notify the writer we're cloning the Arc, so it waits before releasing it.
        cnt.fetch_add(1, Relaxed);
        let arc = unsafe { &*ptr.load(Relaxed) }.clone();
        cnt.fetch_sub(1, Relaxed);
        arc
    }

    pub fn new(obj: T) -> SyncCow<T> {
        let red = Box::new(Arc::new(obj.clone()));
        let green = Box::new(Arc::new(obj.clone()));
        SyncCow {
            // moooo
            latest: AtomicUsize::new(0),
            write_lock: Mutex::new(()),
            atomic_red: (AtomicPtr::new(Box::into_raw(red)), AtomicUsize::new(0)),
            atomic_green: (AtomicPtr::new(Box::into_raw(green)), AtomicUsize::new(0)),
        }
    }
}

impl<T: Clone> Drop for SyncCow<T> {
    fn drop(&mut self) {
        // The Arcs are released Boxes, so we need to make sure they're freed again
        let _ = unsafe { Box::from_raw(self.atomic_red.0.load(Relaxed)) };
        let _ = unsafe { Box::from_raw(self.atomic_green.0.load(Relaxed)) };
    }
}
