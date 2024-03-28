# ECS

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
