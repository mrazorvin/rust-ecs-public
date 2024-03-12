macro_rules! ctx {
  ($d:tt, $macro:ident, $class:ident { $($name:ident: $type:ty),* }) => {
      #[cfg(test)]
      #[derive(::std::default::Default)]
      pub(crate) struct $class {
        $(pub $name: $crate::ecs::testing::Placeholder<$type>),*
      }

      paste::paste! {
          #[cfg(test)]
          use ::std::sync::{atomic as at};

          #[cfg(test)]
          #[allow(non_upper_case_globals)]
          pub(crate) static [<ctx_func_lock_ $macro>]: at::AtomicU64 = at::AtomicU64::new(0);
          #[cfg(test)]
          #[allow(non_upper_case_globals)]
          pub(crate) static [<ctx_state_lock_ $macro>]: at::AtomicU64 = at::AtomicU64::new(0);
          #[cfg(test)]
          #[allow(non_upper_case_globals)]
          pub(crate) static [<ctx_state_type $macro>]:  at::AtomicIsize = at::AtomicIsize::new(0);
          #[cfg(test)]
          #[allow(non_upper_case_globals)]
          pub(crate) static mut [<ctx_state_ $class>]: ::std::option::Option<$class> = ::std::option::Option::None;



          #[cfg(test)]
          pub(crate) struct [<$class ContextGuard>] {}

          #[cfg(test)]
          impl !Send for [<$class ContextGuard>] {}

          #[cfg(test)]
          impl [<$class ContextGuard>] {
            pub(crate) fn new() -> Self {
              while [<ctx_func_lock_ $macro>].compare_exchange(0, 1, at::Ordering::AcqRel, at::Ordering::Acquire).is_err() {}

              let result = [<ctx_state_type $macro>].compare_exchange(0, -1, at::Ordering::AcqRel, at::Ordering::Acquire);
              if (result.is_err() && result.unwrap_err() == 1) {
                panic!("ContextGuard# function already used in immutable context")
              }

              Self {}
            }

            pub(crate) fn init<T>(&self, init: &dyn Fn(&mut $class) -> T) {
              while [<ctx_state_lock_ $macro>].compare_exchange(0, 1, at::Ordering::AcqRel, at::Ordering::Acquire).is_err() {}

              if unsafe { [<ctx_state_ $class>].is_none() } {
                unsafe { [<ctx_state_ $class>] = ::std::option::Option::Some(::std::default::Default::default()) }
              }
              init(unsafe { [<ctx_state_ $class>].as_mut() }.unwrap());

              [<ctx_state_lock_ $macro>].store(0, at::Ordering::Release);
            }
          }

          #[cfg(test)]
          impl Drop for [<$class ContextGuard>] {
            fn drop(&mut self) {
              while [<ctx_state_lock_ $macro>].compare_exchange(0, 1, at::Ordering::AcqRel, at::Ordering::Acquire).is_err() {}

              unsafe { [<ctx_state_ $class>] = ::std::option::Option::None };
              [<ctx_state_lock_ $macro>].store(0, at::Ordering::Release);

              [<ctx_func_lock_ $macro>].store(0, at::Ordering::Release);
            }
          }



          #[cfg(test)]
          pub(crate) fn [<$macro:lower _guard>]() -> [<$class ContextGuard>] {
              [<$class ContextGuard>]::new()
          }

          macro_rules! [<$macro _ctx>] {
            ($prod:expr, $d($func:tt)*) => {{
                #[allow(unused_variables)]
                #[cfg(not(test))]
                let result = $prod;

                #[cfg(test)]
                let result = {
                  let result = [<ctx_state_type $macro>].compare_exchange(0, 1, at::Ordering::AcqRel, at::Ordering::Acquire);

                  if (result.is_ok() || result.unwrap_err() == 1) {
                    $prod
                  } else {
                    paste::paste! {
                      let func = $d($func)*;
                      while [<ctx_state_lock_ $macro>].compare_exchange(0, 1, at::Ordering::AcqRel, at::Ordering::Acquire).is_err() {}
                      let r = func(unsafe { [<ctx_state_ $class>].as_mut() }.expect("ContextGuard# context must be initialized"));
                      [<ctx_state_lock_ $macro>].store(0, at::Ordering::Release);
                      r
                    }
                  }
                };

                result
            }}
          }
      }
  };
}

pub(crate) use ctx;

#[test]
#[should_panic(expected = "ContextGuard# context must be initialized")]
fn context_panic_on_exclisive_invocation_type() {
    ctx!($, func, Context {});

    fn exclusive_invocation() {
        return func_ctx!((), |_: &mut Context| ());
    }

    let guard = func_guard();

    guard.init(&|_| {}); //    1. guard.init() mark context as exclusive
    exclusive_invocation(); // 2. invoking fn success, because context still existed
    drop(guard); //            3. droppping guard, clear current context
    exclusive_invocation(); // 4. invoking fn paincs, because context must exists
}

#[test]
#[should_panic(expected = "ContextGuard# function already used in immutable context")]
fn context_panic_on_shared_invocation_type() {
    ctx!($, func, Context {});

    fn shared_invocation() {
        return func_ctx!((), |_: &mut Context| panic!("ContextGuard# must never happen"));
    }

    shared_invocation(); // 1. invoking function multiple times is ok, with shared context type
    shared_invocation(); // 2. invoking function multiple times is ok, with shared context type
    func_guard(); //                        3. init exclusive context panics, since context already marked as shared
}

#[test]
#[cfg(not(miri))]
fn context_shared_invocation_type() {
    use std::time::Duration;

    ctx!($, func, Context {});

    fn exclusive_func() -> i32 {
        std::thread::sleep(Duration::from_millis(10));
        return func_ctx!(0, |_: &mut Context| 1);
    }

    let parent_guard = func_guard();
    parent_guard.init(&|_| {});

    static mut RESULT: [[i32; 2]; 2] = [[0, 0], [0, 0]];

    let child_thread = std::thread::spawn(|| {
        let child_guard = func_guard();
        child_guard.init(&|_| {});
        unsafe { RESULT[1] = [2, exclusive_func()] };
    });

    std::thread::sleep(Duration::from_millis(100));

    // 1. child_thread can't change global RESULT, since main guard
    //    prevent's new guard creation
    assert_eq!(unsafe { RESULT }, [[0, 0], [0, 0]]);

    // 2. dropping main guard, unlock other guards
    drop(parent_guard);

    // 3. child_thread can freely modify result on background
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(unsafe { RESULT }, [[0, 0], [2, 1]]);

    child_thread.join().expect("ContextGuard# thread is poisoned");
}

#[test]
fn context_exlusive_invocation_type() {
    use std::time::Duration;

    ctx!($, func, Context {});

    fn exclusive_func() -> i32 {
        std::thread::sleep(Duration::from_millis(1));
        return func_ctx!(0, |_: &mut Context| 1);
    }

    let child_thread = std::thread::spawn(|| {
        for _ in 0..33 {
            exclusive_func();
        }
    });

    for _ in 0..33 {
        exclusive_func();
    }

    child_thread.join().expect("ContextGuard# thread is poisoned")
}
