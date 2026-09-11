#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod engine;
#[cfg(test)]
mod smoke;
mod updates;
mod versions;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::Manager;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub active: String,
    pub versions: Vec<versions::Installed>,
    pub presets: Vec<Value>,
}
pub struct Core {
    pub dir: PathBuf,
    pub config: Mutex<Config>,
    pub jobs: Mutex<Vec<engine::Job>>,
    pub installing: Mutex<String>,
    pub version_lock: Mutex<()>,
    pub shutdown: AtomicBool,
}
type Shared = Arc<Core>;
type Result<T> = std::result::Result<T, String>;
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
impl Core {
    pub fn save(&self, c: &Config) -> Result<()> {
        let tmp = self.dir.join("config.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(c).map_err(err)?).map_err(err)?;
        std::fs::rename(tmp, self.dir.join("config.json")).map_err(err)
    }
    pub fn binary(&self) -> Result<PathBuf> {
        let c = self.config.lock().map_err(err)?;
        c.versions
            .iter()
            .find(|v| v.id == c.active)
            .map(|v| PathBuf::from(&v.path))
            .ok_or("Installez ou importez FFmpeg dans Versions.".into())
    }
    pub fn history(&self) {
        if let Ok(jobs) = self.jobs.lock() {
            if let Ok(bytes) = serde_json::to_vec(&*jobs) {
                let _ = std::fs::write(self.dir.join("history.json"), bytes);
            }
        }
    }
}
#[tauri::command]
fn snapshot(state: tauri::State<Shared>) -> Result<Value> {
    Ok(
        serde_json::json!({"config":*state.config.lock().map_err(err)?,"jobs":*state.jobs.lock().map_err(err)?,"installing":*state.installing.lock().map_err(err)?}),
    )
}
#[tauri::command]
fn save_presets(presets: Vec<Value>, state: tauri::State<Shared>) -> Result<()> {
    if presets.len() > 500 {
        return Err("Maximum 500 presets".into());
    }
    for p in &presets {
        if !p["id"].is_string() || !p["name"].is_string() || !p["data"].is_object() {
            return Err("Preset invalide".into());
        }
    }
    let mut c = state.config.lock().map_err(err)?;
    let mut next = c.clone();
    next.presets = presets;
    state.save(&next)?;
    *c = next;
    Ok(())
}
#[tauri::command]
async fn probe(path: String, state: tauri::State<'_, Shared>) -> Result<Value> {
    let bin = state.binary()?;
    tauri::async_runtime::spawn_blocking(move || engine::probe(&bin, &path))
        .await
        .map_err(err)?
}
#[tauri::command]
async fn catalog() -> Result<Vec<versions::Release>> {
    tauri::async_runtime::spawn_blocking(versions::catalog)
        .await
        .map_err(err)?
}
#[tauri::command]
async fn install_version(tag: String, state: tauri::State<'_, Shared>) -> Result<()> {
    let s = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || versions::install(s, tag))
        .await
        .map_err(err)?
}
#[tauri::command]
async fn import_version(path: String, state: tauri::State<'_, Shared>) -> Result<()> {
    let s = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || versions::import(s, path))
        .await
        .map_err(err)?
}
#[tauri::command]
fn select_version(id: String, state: tauri::State<Shared>) -> Result<()> {
    let _guard = state.version_lock.lock().map_err(err)?;
    let mut c = state.config.lock().map_err(err)?;
    if !c.versions.iter().any(|v| v.id == id) {
        return Err("Version inconnue".into());
    }
    let mut next = c.clone();
    next.active = id;
    state.save(&next)?;
    *c = next;
    Ok(())
}
#[tauri::command]
fn remove_version(id: String, state: tauri::State<Shared>) -> Result<()> {
    versions::remove(state.inner(), &id)
}
#[tauri::command]
fn enqueue(
    plans: Vec<Vec<String>>,
    output: String,
    duration: f64,
    state: tauri::State<Shared>,
) -> Result<String> {
    engine::enqueue(state.inner(), plans, output, duration)
}
#[tauri::command]
fn cancel_job(id: String, state: tauri::State<Shared>) -> Result<()> {
    let mut jobs = state.jobs.lock().map_err(err)?;
    let j = jobs
        .iter_mut()
        .find(|j| j.id == id)
        .ok_or("Tâche inconnue")?;
    if j.status == "queued" {
        j.status = "cancelled".into()
    } else if j.status == "running" {
        j.cancel = true
    }
    drop(jobs);
    state.history();
    Ok(())
}
#[tauri::command]
fn reveal_output(path: String) -> Result<()> {
    let p = PathBuf::from(path);
    if !p.is_absolute() || !p.exists() {
        return Err("Fichier introuvable".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", p.display()))
            .spawn()
            .map_err(err)?;
    }
    Ok(())
}
fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let config = if dir.join("config.json").exists() {
                serde_json::from_slice(&std::fs::read(dir.join("config.json"))?)?
            } else {
                Config::default()
            };
            let mut jobs: Vec<engine::Job> = std::fs::read(dir.join("history.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            for j in &mut jobs {
                if j.status == "running" || j.status == "queued" {
                    j.status = "interrupted".into();
                    j.logs.push(
                        "Application interrompue : relancez explicitement cette tâche.".into(),
                    )
                }
            }
            let core = Arc::new(Core {
                dir,
                config: Mutex::new(config),
                jobs: Mutex::new(jobs),
                installing: Mutex::new(String::new()),
                version_lock: Mutex::new(()),
                shutdown: AtomicBool::new(false),
            });
            engine::start_worker(core.clone());
            app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            save_presets,
            probe,
            catalog,
            install_version,
            import_version,
            select_version,
            remove_version,
            enqueue,
            cancel_job,
            reveal_output,
            updates::check_app_update,
            updates::open_release
        ])
        .build(tauri::generate_context!())
        .expect("Impossible de démarrer Encodeck");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            app.state::<Shared>().shutdown.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(250));
        }
    });
}
