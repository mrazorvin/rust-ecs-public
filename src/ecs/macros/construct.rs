#[macro_export]
macro_rules! construct {
  ($entity:expr, ( $($var:ident),* ), $($prop:ident: $val:expr),*) => {{
        ::paste::paste! {
            #[allow(unused, non_snake_case, invalid_value)]
            {
                let mut entity = $entity;
                $(let [<missing_field__ $prop>] = ();)*
                $(let [<missing_field__ $var>] = [<missing_field__ $var>];)*
                $(entity.[<set_ $prop>]($val);)*
                if false {
                    panic!();
                    $(entity.$prop();)*
                }
                 entity
            }
        }
  }};
  ($ty:ident { $($var:ident),* }, $($prop:ident: $val:expr),*) => {{
      ::paste::paste! {
          #[allow(unused, non_snake_case, invalid_value)]
          {
              $(let [<$ty _missing_field__ $prop>] = $val;)*
              if false {
                  let mut defaults: $ty = unsafe { ::std::mem::zeroed() };
                  panic!();
                  $(defaults.$prop = [<$ty _missing_field__ $prop>];)*
              }

              let mut result = $ty {
                  $([<$var>]:[<$ty _missing_field__ $var>],)*
              };

              // it's im possible to skip initiation for some fields
              // we need somehow remove duplicated/non setters fields

              result
          }
      }
  }};
}

// It's possible to improve constructor macro with tt_ident
// macro_rules! test {
//     ($($id:ident),*) => {{
//         let name = 0;
//           macro_tt_utils::tt_ident! {
//             if ($($id == "name")&&*) {
//               paste::paste! {
//                 let name = 2;
//               }
//             } else {
//               let name = "result";
//             }
//         }
//         name
//     }};
// }

// Example
//
// #[derive(Debug)]
// struct App {
//     next: Option<Box<App>>,
//     name: Name,
//     version: u32,
// }
//
// impl App {
//     pub fn set_name(&mut self, name: Name) {
//         self.name = name;
//     }
//
//     pub fn set_version(&mut self, version: u32) {
//         self.version = version;
//     }
// }
//
// macro_rules! new_app {
//     ($ty:ident { $($props:tt)* }) => {
//       $crate::ecs::construct!($ty { next, version, name }, name: Name::default(), $($props)*)
//     };
// }
//
// macro_rules! new_name {
//     ($ty:ident { $($props:tt)* }) => {
//         $crate::ecs::construct!($ty::default(), ( name ), $($props)*)
//     };
// }
//
// impl App {}
//
// #[derive(Debug, Default)]
// struct Name {
//     name: &'static str,
// }
//
// impl Name {
//     fn name(self) -> &'static str {
//         self.name
//     }
//
//     fn set_name(&mut self, name: &'static str) {
//         self.name = name;
//     }
// }
//
// fn main() {
//     let app = new_app!(App {
//         name: new_name!(Name { name: "2" }),
//         version: 4,
//         next: Some(Box::new(new_app!(App { version: 3, next: None })))
//     });
//     println!("{:?}", app);
// }
//
//
