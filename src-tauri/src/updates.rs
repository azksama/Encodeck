use super::*;
pub const REPOSITORY: &str = "azksama/Encodeck";
#[tauri::command]
pub async fn check_app_update() -> Result<Value> {
    tauri::async_runtime::spawn_blocking(||{
 let client=reqwest::blocking::Client::builder().user_agent("Encodeck/1.0.0").timeout(Duration::from_secs(20)).build().map_err(err)?;
 let response=client.get(format!("https://api.github.com/repos/{REPOSITORY}/releases/latest")).send().map_err(err)?;
 if response.status()==reqwest::StatusCode::NOT_FOUND{return Err("Aucune version publique publiée pour le moment.".into())}
 let release:Value=response.error_for_status().map_err(err)?.json().map_err(err)?;
 let tag=release["tag_name"].as_str().ok_or("Version GitHub invalide")?;let version=tag.trim_start_matches('v');
 let latest=semver::Version::parse(version).map_err(err)?;let current=semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(err)?;
 Ok(serde_json::json!({"available":latest>current,"version":version,"notes":release["body"].as_str().unwrap_or(""),"url":format!("https://github.com/{REPOSITORY}/releases")}))
 }).await.map_err(err)?
}
#[tauri::command]
pub fn open_release() -> Result<()> {
    let url = format!("https://github.com/{REPOSITORY}/releases");
    #[cfg(windows)]
    {
        engine::command(std::path::Path::new("explorer.exe"))
            .arg(&url)
            .spawn()
            .map_err(err)?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(err)?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(err)?;
    }
    Ok(())
}
