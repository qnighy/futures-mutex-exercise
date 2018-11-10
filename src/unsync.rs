use std::cell::{Cell, UnsafeCell};

pub struct Mutex<T: ?Sized> {
    locked: Cell<bool>,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(inner: T) -> Self {
        Self {
            locked: Cell::new(false),
            data: UnsafeCell::new(inner),
        }
    }
}
