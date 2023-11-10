#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::{State};

use std::{thread, time};

const SLEEP_MS: time::Duration = time::Duration::from_millis(500);

struct Config {
  testing: bool,
  typing_key: String,
  erase_key: String
}

struct ConfigWrapper {
  config: Mutex<Config>,
}

#[tauri::command]
async fn set_typing_key(state: State<'_, ConfigWrapper>, value: &str) -> Result<(), ()> {
  thread::sleep(SLEEP_MS);
  let mut config = state.config.lock().unwrap();
  config.typing_key = value.to_string();
  Ok(())
}

#[tauri::command]
async fn set_erase_key(state: State<'_, ConfigWrapper>, value: &str) -> Result<(), ()> {
  thread::sleep(SLEEP_MS);
  let mut config = state.config.lock().unwrap();
  config.erase_key = value.to_string();
  Ok(())
}

#[tauri::command]
async fn set_testing(state: State<'_, ConfigWrapper>, value: bool) -> Result<(), ()> {
  thread::sleep(SLEEP_MS);
  let mut config = state.config.lock().unwrap();
  config.testing = value;
  Ok(())
}

fn main() {
  tauri::Builder::default()
    .manage(ConfigWrapper {
        config: Mutex::new(Config {
            testing:    false,
            typing_key: "X".to_string(),
            erase_key:  "Backspace".to_string()
        })})
    .invoke_handler(tauri::generate_handler![
        set_typing_key,
        set_erase_key,
        set_testing
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
