use std::cell::{Cell, UnsafeCell};
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::{LockResult, PoisonError, TryLockError, TryLockResult};
use std::thread;

use futures::prelude::*;
use futures::task::{LocalWaker, Poll};

pub struct Mutex<T: ?Sized> {
    locked: Cell<bool>,
    poisoned: Cell<bool>,
    waiters: Cell<Vec<LocalWaker>>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(inner: T) -> Self {
        Self {
            locked: Cell::new(false),
            poisoned: Cell::new(false),
            waiters: Cell::new(Vec::new()),
            data: UnsafeCell::new(inner),
        }
    }

    pub fn into_inner(self) -> LockResult<T> {
        let Self { poisoned, data, .. } = self;
        let poisoned = poisoned.into_inner();
        let inner = data.into_inner();
        if poisoned {
            Err(PoisonError::new(inner))
        } else {
            Ok(inner)
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn lock(&self) -> MutexAcquire<'_, T> {
        MutexAcquire { mutex: self }
    }
    pub fn poll_lock(&self, lw: &LocalWaker) -> Poll<LockResult<MutexGuard<'_, T>>> {
        if self.locked.get() {
            let mut waiters = self.waiters.replace(Vec::new());
            waiters.push(lw.clone());
            self.waiters.replace(waiters);
            return Poll::Pending;
        }

        let guard = MutexGuard::new(self);
        if self.poisoned.get() {
            Poll::Ready(Err(PoisonError::new(guard)))
        } else {
            Poll::Ready(Ok(guard))
        }
    }

    pub fn try_lock(&self) -> TryLockResult<MutexGuard<'_, T>> {
        if self.locked.get() {
            return Err(TryLockError::WouldBlock);
        }

        let guard = MutexGuard::new(self);
        if self.poisoned.get() {
            Err(PoisonError::new(guard).into())
        } else {
            Ok(guard)
        }
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned.get()
    }

    pub fn get_mut(&mut self) -> LockResult<&mut T> {
        let inner = unsafe { &mut *self.data.get() };
        if self.poisoned.get() {
            Err(PoisonError::new(inner))
        } else {
            Ok(inner)
        }
    }
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    mutex: &'a Mutex<T>,
    is_panicking: bool,
}

impl<'a, T: ?Sized + 'a> MutexGuard<'a, T> {
    fn new(mutex: &'a Mutex<T>) -> Self {
        mutex.locked.set(true);
        Self {
            mutex,
            is_panicking: thread::panicking(),
        }
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
        if !self.is_panicking && thread::panicking() {
            self.mutex.poisoned.set(true);
        }

        let mut waiters = self.mutex.waiters.replace(Vec::new());
        for waiter in waiters.drain(..) {
            waiter.wake();
        }
        self.mutex.waiters.replace(waiters);
    }
}

pub struct MutexAcquire<'a, T: ?Sized + 'a> {
    mutex: &'a Mutex<T>,
}

impl<'a, T: ?Sized + 'a> Future for MutexAcquire<'a, T> {
    type Output = LockResult<MutexGuard<'a, T>>;
    fn poll(self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Self::Output> {
        self.mutex.poll_lock(lw)
    }
}
