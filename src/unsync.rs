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

    pub fn into_inner(self) -> T {
        let Self { data, .. } = self;
        let inner = data.into_inner();
        inner
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn get_mut(&mut self) -> &mut T {
        let inner = unsafe { &mut *self.data.get() };
        inner
    }
}
