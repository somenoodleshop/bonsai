use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Result;
use std::io::prelude::*;
use serde_json::{Value, json};
use std::sync::Mutex;
use tauri::Manager;

struct State {
  data: Mutex<HashMap<String, Value>>,
}

pub fn write_file(file_name: &str, content: Value) -> Result<()> {
  let data = json!(content);
  File::create(file_name)?;
  fs::write(file_name, data.to_string())?;
  Ok(())
}

fn sources_reducer(state: Value, event: &str, payload: &str) -> Value {
  let mut new_state = state.clone();
  match event {
    "add_source" => {
        let id = new_state.as_object().unwrap().len();
        let source: Value = serde_json::from_str(payload).unwrap();
        new_state.as_object_mut().unwrap().insert(
            id.to_string(),
            json!({
                "source": source,
                "id": id,
            }),
        );
        return new_state
    }
    _ => {
      println!("Unknown command: {}", event);
    }
  }
  state
}

pub fn read_file(file_name: &str, default_value: Value) -> Result<String> {
  let mut buffer = String::new();
  let mut file = match File::open(file_name) {
    Ok(file) => file,
    Err(_) => {
      write_file(file_name, default_value)?;
      File::open(file_name)?
    }
  };

  file.read_to_string(&mut buffer)?;
  Ok(buffer)
}

fn state_identity(state: Value, event: &str, payload: &str) -> Value {
  // This function is a placeholder for state that does not change
  // It simply returns the state as is, without modification
  println!("State identity called with event: {}, payload: {}", event, payload);
  state
}

fn readings_reducer(state: Value, event: &str, payload: &str) -> Value {
  let mut new_state = state.clone();
  match event {
    "add_reading" => {
        new_state.as_array_mut().unwrap().push(json!({ "reading": payload }));
    }
    _ => {
      println!("Unknown command: {}", event);
    }
  }
  new_state
}

fn get_state_keys() -> HashMap<String, (Value, fn(Value, &str, &str) -> Value)> {
  let mut keys = HashMap::new();

  keys.insert("sources".to_string(), (json!({}), sources_reducer as fn(Value, &str, &str) -> Value));
  keys.insert("readings".to_string(), (json!({}), readings_reducer as fn(Value, &str, &str) -> Value));

  keys
}

#[tauri::command]
fn dispatch(event: String, payload: Option<String>, state: tauri::State<State>) -> String {
  let mut updated_data = state.data.lock().unwrap().clone();
  let readable_data = updated_data.clone();

  let state_keys = get_state_keys();

  for (key, value) in readable_data.iter() {
    // this needs to be different for each key--state modification is a reducer
    let (_initial_value, reducer) = state_keys.get(key).unwrap();
    let updated_value = reducer(value.clone(), &event, &payload.clone().unwrap_or_default().clone());
    updated_data.insert(key.clone(), updated_value.clone());
    write_file(&format!("{}.json", key), json!(updated_value.clone())).expect("Failed to write to file");
  }
  *state.data.lock().unwrap() = updated_data.clone();
  serde_json::to_string(&updated_data).unwrap()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {

      let state = app.state::<State>();
      let mut data = state.data.lock().unwrap();

      for (name, attributes) in get_state_keys().iter() {
          let (initial_state, _modify_fn) = attributes;
          let initial_data = read_file(&format!("{}.json", name), initial_state.clone()).unwrap();
          let initial_json: Value = serde_json::from_str(&initial_data).unwrap();
          data.insert(name.to_string(), initial_json);
      }

      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .manage(State { data: Mutex::new(HashMap::new()) })
    .invoke_handler(tauri::generate_handler![dispatch])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
