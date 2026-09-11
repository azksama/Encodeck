use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
#[derive(Clone, Serialize, Deserialize)]
pub struct Installed {
    pub id: String,
    pub name: String,
    pub path: String,
    pub managed: bool,
    pub checksum: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    pub name: String,
    pub url: String,
    pub size: u64,
    pub digest: String,
    #[serde(default)]
    pub probe_url: String,
    #[serde(default)]
    pub probe_digest: String,
}
fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent("FFmpeg-Commander-Desktop/1.0")
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(900))
        .build()
        .map_err(err)
}
pub fn catalog() -> Result<Vec<Release>> {
    if !cfg!(windows) {
        return portable_catalog();
    }
    let data: Vec<Value> = client()?
        .get("https://api.github.com/repos/GyanD/codexffmpeg/releases?per_page=20")
        .send()
        .map_err(err)?
        .error_for_status()
        .map_err(err)?
        .json()
        .map_err(err)?;
    Ok(data
        .iter()
        .filter(|r| r["prerelease"] == false && r["draft"] == false)
        .filter_map(|r| {
            let a = r["assets"].as_array()?.iter().find(|a| {
                a["name"]
                    .as_str()
                    .is_some_and(|n| n.ends_with("essentials_build.zip"))
            })?;
            Some(Release {
                tag: r["tag_name"].as_str()?.into(),
                name: a["name"].as_str()?.into(),
                url: a["browser_download_url"].as_str()?.into(),
                size: a["size"].as_u64()?,
                digest: a["digest"].as_str().unwrap_or("").into(),
                probe_url: String::new(),
                probe_digest: String::new(),
            })
        })
        .collect())
}
fn version(path: &Path) -> Result<String> {
    let out = engine::command(path)
        .arg("-version")
        .output()
        .map_err(err)?;
    if !out.status.success() {
        return Err("Le binaire FFmpeg ne démarre pas".into());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().unwrap_or("");
    if !line.starts_with("ffmpeg version ") {
        return Err("Ce fichier n’est pas FFmpeg".into());
    }
    Ok(line.split_whitespace().nth(2).unwrap_or("inconnue").into())
}
pub fn import(s: Shared, path: String) -> Result<()> {
    let _guard = s.version_lock.lock().map_err(err)?;
    let p = fs::canonicalize(path).map_err(err)?;
    let name = version(&p)?;
    if !p
        .with_file_name(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        })
        .exists()
    {
        return Err("ffprobe doit être dans le même dossier que FFmpeg.".into());
    }
    let mut c = s.config.lock().map_err(err)?;
    let mut next = c.clone();
    let path = p.to_string_lossy().to_string();
    if let Some(v) = next.versions.iter().find(|v| v.path == path) {
        next.active = v.id.clone()
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        next.versions.push(Installed {
            id: id.clone(),
            name,
            path,
            managed: false,
            checksum: String::new(),
        });
        next.active = id
    }
    s.save(&next)?;
    *c = next;
    Ok(())
}
pub fn install(s: Shared, tag: String) -> Result<()> {
    if !cfg!(windows) {
        return install_portable(s, tag);
    }
    if !cfg!(target_arch = "x86_64") {
        return Err("Importez une version locale de FFmpeg sur Windows ARM.".into());
    }
    let _guard = s
        .version_lock
        .try_lock()
        .map_err(|_| "Une opération de version est déjà en cours.")?;
    *s.installing.lock().map_err(err)? = "Recherche de la version…".into();
    let result = (|| {
        let release = catalog()?
            .into_iter()
            .find(|r| r.tag == tag)
            .ok_or("Version absente du catalogue")?;
        if !tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        {
            return Err("Identifiant invalide".into());
        }
        if s.config
            .lock()
            .map_err(err)?
            .versions
            .iter()
            .any(|v| v.id == tag)
        {
            return Err("Cette version est déjà installée.".into());
        }
        if !release
            .url
            .starts_with("https://github.com/GyanD/codexffmpeg/releases/download/")
        {
            return Err("Source de téléchargement invalide".into());
        }
        let client = client()?;
        let mut response = client
            .get(&release.url)
            .send()
            .map_err(err)?
            .error_for_status()
            .map_err(err)?;
        let root = s.dir.join("versions");
        fs::create_dir_all(&root).map_err(err)?;
        let staging = root.join(format!(".install-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&staging).map_err(err)?;
        let result = (|| {
            let zip_path = staging.join("download.zip");
            let mut zip_file = fs::File::create(&zip_path).map_err(err)?;
            let mut hash = Sha256::new();
            let mut buffer = [0; 65536];
            let mut downloaded = 0u64;
            loop {
                let n = response.read(&mut buffer).map_err(err)?;
                if n == 0 {
                    break;
                }
                downloaded += n as u64;
                if downloaded > 600_000_000 {
                    return Err("Archive trop volumineuse".into());
                }
                hash.update(&buffer[..n]);
                zip_file.write_all(&buffer[..n]).map_err(err)?;
                *s.installing.lock().map_err(err)? = format!(
                    "Téléchargement · {} / {} Mo",
                    downloaded / 1_000_000,
                    release.size / 1_000_000
                );
            }
            drop(zip_file);
            let checksum = format!("{:x}", hash.finalize());
            let expected = if let Some(d) = release.digest.strip_prefix("sha256:") {
                d.to_string()
            } else {
                let text = client
                    .get(format!(
                        "https://www.gyan.dev/ffmpeg/builds/packages/{}.sha256",
                        release.name
                    ))
                    .send()
                    .map_err(err)?
                    .error_for_status()
                    .map_err(err)?
                    .text()
                    .map_err(err)?;
                text.split_whitespace().next().unwrap_or("").to_lowercase()
            };
            if checksum != expected {
                return Err("La vérification SHA-256 a échoué. Installation abandonnée.".into());
            }
            *s.installing.lock().map_err(err)? = "SHA-256 vérifié · Extraction…".into();
            let mut archive =
                zip::ZipArchive::new(fs::File::open(&zip_path).map_err(err)?).map_err(err)?;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).map_err(err)?;
                let name = file.enclosed_name().ok_or("Chemin d’archive dangereux")?;
                if file.is_dir() {
                    continue;
                }
                if file.size() > 600_000_000 {
                    return Err("Fichier trop volumineux".into());
                }
                let dest = staging.join(name);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent).map_err(err)?
                }
                std::io::copy(&mut file, &mut fs::File::create(dest).map_err(err)?).map_err(err)?;
            }
            fs::remove_file(zip_path).map_err(err)?;
            let binary = find_binary(&staging).ok_or("ffmpeg.exe introuvable dans l’archive")?;
            let name = version(&binary)?;
            if !binary.with_file_name("ffprobe.exe").exists() {
                return Err("ffprobe absent".into());
            }
            let relative = binary.strip_prefix(&staging).map_err(err)?.to_path_buf();
            let dest = root.join(&tag);
            if dest.exists() {
                return Err(
                    "Dossier de version déjà présent. Importez son binaire pour le récupérer."
                        .into(),
                );
            }
            fs::rename(&staging, &dest).map_err(err)?;
            let mut c = s.config.lock().map_err(err)?;
            let mut next = c.clone();
            next.versions.push(Installed {
                id: tag.clone(),
                name,
                path: dest.join(relative).to_string_lossy().to_string(),
                managed: true,
                checksum,
            });
            next.active = tag.clone();
            s.save(&next)?;
            *c = next;
            Ok(())
        })();
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    })();
    *s.installing.lock().map_err(err)? = String::new();
    result
}
fn find_binary(dir: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.file_name()?.to_str()? == "ffmpeg.exe" {
            return Some(p);
        }
        if p.is_dir() {
            if let Some(found) = find_binary(&p) {
                return Some(found);
            }
        }
    }
    None
}
fn portable_catalog() -> Result<Vec<Release>> {
    let platform = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return Err("Plateforme non prise en charge".into());
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        return Err("Architecture non prise en charge".into());
    };
    let data: Vec<Value> = client()?
        .get("https://api.github.com/repos/descriptinc/ffmpeg-ffprobe-static/releases?per_page=20")
        .send()
        .map_err(err)?
        .error_for_status()
        .map_err(err)?
        .json()
        .map_err(err)?;
    Ok(data
        .iter()
        .filter(|r| r["draft"] == false)
        .filter_map(|r| {
            let assets = r["assets"].as_array()?;
            let a = assets
                .iter()
                .find(|a| a["name"] == format!("ffmpeg-{platform}-{arch}"))?;
            let probe = assets
                .iter()
                .find(|a| a["name"] == format!("ffprobe-{platform}-{arch}"))?;
            Some(Release {
                tag: r["tag_name"].as_str()?.into(),
                name: format!("Descript · {platform} {arch}"),
                url: a["browser_download_url"].as_str()?.into(),
                size: a["size"].as_u64()? + probe["size"].as_u64()?,
                digest: a["digest"].as_str().unwrap_or("").into(),
                probe_url: probe["browser_download_url"].as_str()?.into(),
                probe_digest: probe["digest"].as_str().unwrap_or("").into(),
            })
        })
        .collect())
}
fn install_portable(s: Shared, tag: String) -> Result<()> {
    let _guard = s
        .version_lock
        .try_lock()
        .map_err(|_| "Une installation est déjà en cours")?;
    if !tag
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err("Version invalide".into());
    }
    if s.config
        .lock()
        .map_err(err)?
        .versions
        .iter()
        .any(|v| v.id == tag)
    {
        return Err("Cette version est déjà installée".into());
    }
    *s.installing.lock().map_err(err)? = "Téléchargement de FFmpeg et ffprobe…".into();
    let root = s.dir.join("versions");
    fs::create_dir_all(&root).map_err(err)?;
    let stage = root.join(format!(".install-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        let r = portable_catalog()?
            .into_iter()
            .find(|r| r.tag == tag)
            .ok_or("Version inconnue")?;
        fs::create_dir(&stage).map_err(err)?;
        let client = client()?;
        let mut sums = vec![];
        for (name, url, digest) in [
            ("ffmpeg", r.url, r.digest),
            ("ffprobe", r.probe_url, r.probe_digest),
        ] {
            if !url.starts_with(
                "https://github.com/descriptinc/ffmpeg-ffprobe-static/releases/download/",
            ) {
                return Err("Source invalide".into());
            }
            let mut response = client
                .get(&url)
                .send()
                .map_err(err)?
                .error_for_status()
                .map_err(err)?;
            let target = stage.join(name);
            let mut file = fs::File::create(&target).map_err(err)?;
            let mut hash = Sha256::new();
            let mut buf = [0; 65536];
            let mut count = 0u64;
            loop {
                let n = response.read(&mut buf).map_err(err)?;
                if n == 0 {
                    break;
                }
                count += n as u64;
                if count > 600_000_000 {
                    return Err("Téléchargement trop volumineux".into());
                }
                file.write_all(&buf[..n]).map_err(err)?;
                hash.update(&buf[..n]);
                *s.installing.lock().map_err(err)? = format!("{name} · {} Mo", count / 1_000_000);
            }
            drop(file);
            let sum = format!("{:x}", hash.finalize());
            if let Some(expected) = digest.strip_prefix("sha256:") {
                if expected != sum {
                    return Err("SHA-256 incorrect".into());
                }
            }
            sums.push(format!("{name}:{sum}"));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).map_err(err)?;
            }
        }
        let name = version(&stage.join("ffmpeg"))?;
        let dest = root.join(&tag);
        if dest.exists() {
            return Err("Dossier de version déjà présent".into());
        }
        fs::rename(&stage, &dest).map_err(err)?;
        let mut c = s.config.lock().map_err(err)?;
        let mut next = c.clone();
        next.versions.push(Installed {
            id: tag.clone(),
            name,
            path: dest.join("ffmpeg").to_string_lossy().to_string(),
            managed: true,
            checksum: sums.join(";"),
        });
        next.active = tag;
        s.save(&next)?;
        *c = next;
        Ok(())
    })();
    if stage.exists() {
        let _ = fs::remove_dir_all(&stage);
    }
    *s.installing.lock().map_err(err)? = String::new();
    result
}
pub fn remove(s: &Shared, id: &str) -> Result<()> {
    let _guard = s.version_lock.lock().map_err(err)?;
    let mut c = s.config.lock().map_err(err)?;
    if c.active == id {
        return Err("Sélectionnez une autre version avant de retirer la version active.".into());
    }
    let v = c
        .versions
        .iter()
        .find(|v| v.id == id)
        .ok_or("Version inconnue")?
        .clone();
    if s.jobs
        .lock()
        .map_err(err)?
        .iter()
        .any(|j| j.binary == v.path && (j.status == "queued" || j.status == "running"))
    {
        return Err("Cette version est utilisée par une tâche.".into());
    }
    // Soft removal: managed files remain recoverable in the application's trash.
    if v.managed {
        let root = fs::canonicalize(s.dir.join("versions")).map_err(err)?;
        let dir = fs::canonicalize(root.join(id)).map_err(err)?;
        if dir == root || !dir.starts_with(&root) {
            return Err("Chemin hors du dossier des versions".into());
        }
        let trash = s.dir.join("trash");
        fs::create_dir_all(&trash).map_err(err)?;
        fs::rename(dir, trash.join(format!("{}-{}", id, uuid::Uuid::new_v4()))).map_err(err)?;
    }
    let mut next = c.clone();
    next.versions.retain(|v| v.id != id);
    s.save(&next)?;
    *c = next;
    Ok(())
}
