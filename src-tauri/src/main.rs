// Скрываем консольное окно на Windows в релизной сборке
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    shepot_lib::run()
}
