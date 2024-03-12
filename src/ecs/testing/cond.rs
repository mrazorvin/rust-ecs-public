#[macro_export]
macro_rules! cond {
    (true, $expr:expr) => {
        $expr
    };
    (false, $expr:expr) => {};
    (not true, $expr:expr) => {};
    (not false, $expr:expr) => {
        $expr
    };
}
