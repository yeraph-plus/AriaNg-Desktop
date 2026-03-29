// Prevents a console window from appearing on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ariang_app_lib::run();
}
