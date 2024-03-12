use game::game_wolf_survivors::start_game;

#[cfg(not(target_os = "android"))]
fn main() {
    start_game().unwrap();
}

#[cfg(target_os = "android")]
#[no_mangle]
#[allow(non_snake_case)]
pub fn SDL_main() {
    game::android::android_log_thread::spawn_android_log_thread();
    start_game().unwrap();
}
