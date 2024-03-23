use super::{logger::logger_file_storage_base, system, world};

pub fn integration_tests() -> Vec<system::SysFn> {
    vec![test_fs_and_logger]
}

fn test_fs_and_logger(sys: &mut system::State) -> system::Return {
    system::define!(sys,);
    let assets_path = sdl2::filesystem::pref_path("Demo", "Game")?;
    std::fs::create_dir_all(&assets_path)?;
    logger_file_storage_base(&format!("{assets_path}/test_logs"))?;
    system::OK
}

// Add tests for
// - SpraseSet + Components
// - Metrics performance
