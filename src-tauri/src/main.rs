// Sem console no Windows em release; o modo headless escreve em arquivo, não no stdout.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    reverb_lib::run();
}
