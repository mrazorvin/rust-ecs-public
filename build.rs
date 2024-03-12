fn main() {
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");

    println!(
        r"ccargo:rustc-link-search=native={}",
        "C:\\Users\\Admin\\Documents\\MEGA\\rust-ecs\\target\\aarch64-linux-android\\release\\deps"
    );
    println!("cargo:rustc-link-lib=dylib=SDL2");
}
