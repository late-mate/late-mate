#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused)]

use std::sync::Mutex;
use tauri::{Manager, State, Window};

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
async fn set_testing(window: Window, state: State<'_, ConfigWrapper>, value: bool) -> Result<(), ()> {
  thread::sleep(SLEEP_MS);
  let mut config = state.config.lock().unwrap();

  let app = window.app_handle();
  if !config.testing && value {
    std::thread::spawn(move || {
      let state = app.state::<ConfigWrapper>();
      while state.config.lock().unwrap().testing {
        let now = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis();
        window.emit("measurement", Measurement { value: now as f64 }).unwrap();
        thread::sleep(SLEEP_MS);
      }
    });
  }
  
  config.testing = value;
  Ok(())
}

// the payload type must implement `Serialize` and `Clone`.
#[derive(Clone, serde::Serialize)]
struct Measurement {
  value: f64,
}

fn setup<'a>(app: &'a mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
  let handle = app.handle();
  // tauri::async_runtime::spawn(async move {
  //   loop {
  //     let now = time::SystemTime::now()
  //         .duration_since(time::UNIX_EPOCH)
  //         .expect("Time went backwards")
  //         .as_millis();
  //     handle.emit_all("measurement", Measurement { value: now as f64 }).unwrap();
  //     thread::sleep(SLEEP_MS);
  //   }
  // });
  Ok(())
}

fn main() {
  tauri::Builder::default()
    .setup(setup)
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
