// could be implmented by any resource that needed end from disposing
pub trait FrameDisposable {
    unsafe fn dispose(&self);
}
