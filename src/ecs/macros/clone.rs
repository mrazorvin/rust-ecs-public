#[macro_export]
macro_rules! clone {
    ($ty:ident { $($props:tt)* }, ($source:ident, $target:ident)) => {{
        clone!($source, $target, $($props)*);
    }};
    ($source:expr, $target:expr, $prop:ident, $($next:tt)*) => {
        $target.$prop = $source.$prop.clone();
        clone!($source, $target, $($next)*);
    };
    ($source:expr, $target:expr, $prop:ident) => {
        $target.$prop = $source.$prop.clone();
    };
    ($source:expr, $target:expr, $prop:ident: $ty:ident { $($nested:tt)* }) => {
        clone!($source.$prop, $target.$prop, $($nested)*);
    };
    ($source:expr, $target:expr, $prop:ident: $ty:ident { $($nested:tt)* }, $($next:tt)*) => {
        clone!($source.$prop, $target.$prop, $($nested)*);
        clone!($source, $target, $($next)*);
    };
}

#[macro_export]
macro_rules! clone_set {
    ($ty:ident { $($props:tt)* }, $source:ident, $target:ident) => {{
        clone_set!($source, $target, $($props)*);
    }};
    ($source:expr, $target:expr, $prop:ident) => {
        ::paste::paste! {
            $target.[<set_ $prop>]($source.$prop());
        }
    };
    ($source:expr, $target:expr, $prop:ident, $($next:tt)*) => {
        ::paste::paste! {
            $target.[<set_ $prop>]($source.$prop());
        }
        clone_set!($source, $target, $($next)*);
    };
    ($source:expr, $target:expr, $prop:ident: $ty:ident { $($nested:tt)* }) => {
        clone_set!($source.$prop, $target.$prop, $($nested)*);
    };
    ($source:expr, $target:expr, $prop:ident: $ty:ident { $($nested:tt)* }, $($next:tt)*) => {
        clone_set!($source.$prop, $target.$prop, $($nested)*);
        clone_set!($source, $target, $($next)*);
    };
}
