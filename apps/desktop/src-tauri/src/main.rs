// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    match meetburner_lib::parse_command(std::env::args().skip(1)) {
        meetburner_lib::Command::Run => meetburner_lib::run(),
        meetburner_lib::Command::UnregisterCameras => {
            std::process::exit(meetburner_lib::unregister_cameras_cli())
        }
    }
}
