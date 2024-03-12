unsafe impl<T> SafeToUpdate for SafeVec<T> {}
struct SafeVec<T> {
    vec: Vec<T>,
}

unsafe trait SafeToUpdate {}
unsafe impl<T: Copy> SafeToUpdate for T {}

#[repr(transparent)]
struct SafetyGuard<T> {
    inner: T,
}

impl<T> SafetyGuard<T> {
    fn get_mut<'a, V: SafeToUpdate>(&'a mut self, func: fn(&mut T) -> &mut V) -> &'a mut V {
        func(&mut self.inner)
    }
}

impl<T> Deref for SafetyGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: SafeToUpdate> DerefMut for SafetyGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
