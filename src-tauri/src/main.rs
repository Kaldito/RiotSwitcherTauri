// Evita la ventana de consola adicional en Windows en release. NO QUITAR.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    riotswitcher_lib::run();
}
