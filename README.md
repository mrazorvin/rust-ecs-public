# ECS

## Docs

- Basic info about currently implemneted pattern, features and concepts: https://docs.google.com/spreadsheets/d/1Zp1RSgAQ-T9jpHwEgUBQo5P1c1JHqDf30X_3NDAJFqU/edit#gid=0

## Commands

- Build    - `cargo run --features release`
- Examples - `cargo run --example log --features release`
- Tests    - `cargo test --lib -Zpanic-abort-tests` or `cargo nextest run --lib`
- Mir      - `cargo miri nextest run -j4 --color=always --lib world`

## Testing

- Unit tests correctly works only in with test-per-process runners `nextest` or `-Zpanic-abort-tests` flag
- `#[cfg(test)]` - compile code only for tests env
- `#[cfg(feature = "release")]`- compile code for tests and production env

## Rust-analyzer

- enable typing for testing & release code `cargo.features = ["release"]`
- use `if (true) {} else {}` to prevent borrowing issues for shared variables

## FAQ

- Integration tests freeze without error? Add `-- --nocapture` paramaters to get panics logs

## Wolf survivors 
```sh
cargo run --example wolf_survivors --features=pc
```

## Android

```
cd C:\Users\Admin\Documents\MEGA\rust-ecs
$Env:SDL="C:\Users\Admin\Documents\MEGA\SDL"


$Env:ANDROID_HOME="C:\Users\Admin\AppData\Local\Android\Sdk"
$Env:ANDROID_NDK_HOME="C:\Users\Admin\Documents\MEGA\Android_NDK" 
$Env:AR_aarch64_linux_android="C:\\Users\\Admin\\Documents\\MEGA\\Android_NDK\\toolchains\\llvm\\prebuilt\\windows-x86_64\\bin\\llvm-ar.exe" 
$Env:CC_aarch64_linux_android="C:\\Users\\Admin\\Documents\\MEGA\\Android_NDK\\toolchains\\llvm\\prebuilt\\windows-x86_64\\bin\\aarch64-linux-android24-clang.cmd"
$Env:CXX_aarch64_linux_android="C:\\Users\\Admin\\Documents\\MEGA\\Android_NDK\\toolchains\\llvm\\prebuilt\\windows-x86_64\\bin\\aarch64-linux-android24-clang++.cmd"

```


