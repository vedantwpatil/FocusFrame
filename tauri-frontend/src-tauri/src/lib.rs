use std::process::Command;
use tauri::State;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct RecordingState(Mutex<Option<std::process::Child>>);

#[derive(Serialize)]
struct Recording {
    name: String,
    path: String,
}

#[tauri::command]
async fn start_recording(
    name: &str, 
    state: State<'_, RecordingState>
) -> Result<(), String> {
    let output_path = format!("output/{}.mp4", name);
    
    let mut child = Command::new("./go-backend/bin/screen_recorder")
        .arg(&output_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    *state.0.lock().unwrap() = Some(child);
    Ok(())
}

#[tauri::command]
async fn stop_recording(state: State<'_, RecordingState>) -> Result<(), String> {
    if let Some(mut child) = state.0.lock().unwrap().take() {
        child.kill().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_recordings() -> Result<Vec<Recording>, String> {
    let entries = std::fs::read_dir("output")
        .map_err(|e| e.to_string())?;
    
    let mut recordings = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".mp4") {
                    recordings.push(Recording {
                        name: name.to_string(),
                        path: entry.path().to_str().unwrap().to_string(),
                    });
                }
            }
        }
    }
    
    Ok(recordings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(RecordingState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            get_recordings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}