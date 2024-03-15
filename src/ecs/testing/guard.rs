use std::sync::atomic::AtomicUsize;

pub struct Guard<'a> {
    pub lock: &'a AtomicUsize,
}

impl<'a> Guard<'a> {
    pub fn clone(&self) -> Self {
        while self
            .lock
            .compare_exchange(
                0,
                1,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {}

        Self { lock: self.lock }
    }
}

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        self.lock.store(0, std::sync::atomic::Ordering::SeqCst)
    }
}
