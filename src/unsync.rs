use std::cell::{Cell, UnsafeCell};
use std::ops::{Deref, DerefMut};

use futures::task::{LocalWaker, Poll};

pub struct Mutex<T: ?Sized> {
    locked: Cell<bool>,
    waiters: Cell<Vec<LocalWaker>>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(inner: T) -> Self {
        Self {
            locked: Cell::new(false),
            waiters: Cell::new(Vec::new()),
            data: UnsafeCell::new(inner),
        }
    }

    pub fn into_inner(self) -> T {
        let Self { data, .. } = self;
        let inner = data.into_inner();
        inner
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn poll_lock(&self, lw: &LocalWaker) -> Poll<MutexGuard<'_, T>> {
        if self.locked.get() {
            let mut waiters = self.waiters.replace(Vec::new());
            waiters.push(lw.clone());
            self.waiters.replace(waiters);
            return Poll::Pending;
        }

        let guard = MutexGuard::new(self);
        Poll::Ready(guard)
    }

    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.locked.get() {
            return None;
        }

        let guard = MutexGuard::new(self);
        Some(guard)
    }

    pub fn get_mut(&mut self) -> &mut T {
        let inner = unsafe { &mut *self.data.get() };
        inner
    }
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    mutex: &'a Mutex<T>,
}

impl<'a, T: ?Sized + 'a> MutexGuard<'a, T> {
    fn new(mutex: &'a Mutex<T>) -> Self {
        mutex.locked.set(true);
        Self { mutex }
    }
}

impl<'a, T: ?Sized + 'a> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized + 'a> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T: ?Sized + 'a> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.set(false);
        let mut waiters = self.mutex.waiters.replace(Vec::new());
        for waiter in waiters.drain(..) {
            waiter.wake();
        }
        self.mutex.waiters.replace(waiters);
    }
}
