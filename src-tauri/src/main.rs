#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::{State};

struct Counter {
    count: Mutex<i64>,
}

#[tauri::command]
fn greet(name: &str) -> String {
   format!("Hello, <em>{}</em>!", name)
}

#[tauri::command]
fn click(state: State<Counter>) -> i64 {
   let mut counter = state.count.lock().unwrap();
   *counter = *counter + 1;
   return *counter;
}

fn main() {
  tauri::Builder::default()
    .manage(Counter {count: Mutex::new(0)})
    .invoke_handler(tauri::generate_handler![greet])
    .invoke_handler(tauri::generate_handler![click])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
