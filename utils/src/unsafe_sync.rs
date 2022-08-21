use core::ops::{Deref, DerefMut};

/// Unsafe wrapper that implements Sync for inner
///
/// # Why
/// Sometimes we don't need to mutate a global or only mutate it to initialize value.
/// Because rustc enforces Sync, it becomes hard to implement those.
///
/// # SAFETY:
/// Never mutate inner after initialization.
pub struct UnsafeSync<T> {
    inner: T,
}

unsafe impl<T> Sync for UnsafeSync<T> {}

impl<T> UnsafeSync<T> {
    /// # SAFETY:
    /// Never mutate inner after calling this
    pub unsafe fn new(value: T) -> Self {
        Self { inner: value }
    }
}

impl<T> Deref for UnsafeSync<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for UnsafeSync<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
