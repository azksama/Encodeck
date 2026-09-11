use super::*;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
};
#[derive(Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub output: String,
    pub binary: String,
    pub plans: Vec<Vec<String>>,
    pub status: String,
    pub progress: f64,
    pub duration: f64,
    pub pass: usize,
    pub logs: Vec<String>,
    pub created: u64,
    #[serde(default)]
    pub cancel: bool,
}
pub fn command(binary: &std::path::Path) -> Command {
    let mut c = Command::new(binary);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
    }
    c
}
pub fn probe(binary: &std::path::Path, path: &str) -> Result<Value> {
    let probe = binary.with_file_name(if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    });
    let mut child = command(&probe)
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
            path,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(err)?;
    let out = child.stdout.take().unwrap();
    let errors = child.stderr.take().unwrap();
    let stdout = thread::spawn(move || {
        use std::io::Read;
        let mut b = Vec::new();
        out.take(8 * 1024 * 1024).read_to_end(&mut b).map(|_| b)
    });
    let stderr = thread::spawn(move || {
        use std::io::Read;
        let mut b = Vec::new();
        errors.take(1024 * 1024).read_to_end(&mut b).map(|_| b)
    });
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(err)? {
            break s;
        }
        if started.elapsed() > Duration::from_secs(25) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("L’analyse du média a dépassé 25 secondes.".into());
        }
        thread::sleep(Duration::from_millis(50))
    };
    let bytes = stdout
        .join()
        .map_err(|_| "Lecture interrompue")?
        .map_err(err)?;
    let errors = stderr
        .join()
        .map_err(|_| "Lecture interrompue")?
        .map_err(err)?;
    if !status.success() {
        return Err(String::from_utf8_lossy(&errors).into());
    }
    serde_json::from_slice(&bytes).map_err(err)
}
pub fn validate(plans: &[Vec<String>], output: &str) -> Result<()> {
    if plans.is_empty() || plans.len() > 2 {
        return Err("Une ou deux passes sont nécessaires.".into());
    }
    if output.trim().is_empty() {
        return Err("Choisissez une sortie.".into());
    }
    for p in plans {
        if p.is_empty() || p.len() > 512 || p.iter().any(|a| a.contains('\0')) {
            return Err("Arguments invalides".into());
        }
        if !p.iter().any(|a| a == "-i") {
            return Err("Entrée manquante".into());
        }
        for pair in p.windows(2) {
            if pair[0] == "-i"
                && (pair[1] == output
                    || (std::fs::canonicalize(&pair[1])
                        .ok()
                        .zip(std::fs::canonicalize(output).ok())
                        .is_some_and(|(a, b)| a == b)))
            {
                return Err("La sortie doit être différente de l’entrée.".into());
            }
        }
    }
    if plans.last().and_then(|p| p.last()).map(String::as_str) != Some(output) {
        return Err("La sortie ne correspond pas à la commande.".into());
    }
    Ok(())
}
pub fn enqueue(
    s: &Shared,
    plans: Vec<Vec<String>>,
    output: String,
    duration: f64,
) -> Result<String> {
    validate(&plans, &output)?;
    let _guard = s.version_lock.lock().map_err(err)?;
    let binary = s.binary()?.to_string_lossy().to_string();
    let id = uuid::Uuid::new_v4().to_string();
    let mut jobs = s.jobs.lock().map_err(err)?;
    if jobs
        .iter()
        .filter(|j| j.status == "queued" || j.status == "running")
        .count()
        >= 50
    {
        return Err("File d’attente pleine (50 tâches).".into());
    }
    if jobs
        .iter()
        .any(|j| (j.status == "queued" || j.status == "running") && j.output == output)
    {
        return Err("Une tâche utilise déjà cette sortie.".into());
    }
    jobs.push(Job {
        id: id.clone(),
        output,
        binary,
        plans,
        status: "queued".into(),
        duration,
        progress: 0.,
        pass: 1,
        logs: vec![],
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        cancel: false,
    });
    while jobs.len() > 100 {
        if let Some(i) = jobs
            .iter()
            .position(|j| j.status != "queued" && j.status != "running")
        {
            jobs.remove(i);
        } else {
            break;
        }
    }
    drop(jobs);
    s.history();
    Ok(id)
}
fn update(s: &Shared, id: &str, f: impl FnOnce(&mut Job)) {
    if let Ok(mut jobs) = s.jobs.lock() {
        if let Some(j) = jobs.iter_mut().find(|j| j.id == id) {
            f(j)
        }
    }
}
fn stopped(s: &Shared, id: &str) -> bool {
    s.shutdown.load(Ordering::SeqCst)
        || s.jobs
            .lock()
            .map(|js| js.iter().any(|j| j.id == id && j.cancel))
            .unwrap_or(true)
}
fn log(s: &Shared, id: &str, line: String) {
    update(s, id, |j| {
        if let Some(time) = line
            .strip_prefix("out_time_us=")
            .and_then(|v| v.parse::<f64>().ok())
        {
            if j.duration > 0. {
                j.progress = (((j.pass - 1) as f64
                    + (time / 1_000_000. / j.duration).clamp(0., 1.))
                    / j.plans.len() as f64
                    * 100.)
                    .min(99.9)
            }
        }
        j.logs.push(line);
        if j.logs.len() > 250 {
            j.logs.remove(0);
        }
    })
}
pub fn start_worker(s: Shared) {
    thread::spawn(move || {
        while !s.shutdown.load(Ordering::SeqCst) {
            let job = {
                let mut jobs = s.jobs.lock().unwrap();
                jobs.iter_mut().find(|j| j.status == "queued").map(|j| {
                    j.status = "running".into();
                    j.clone()
                })
            };
            if let Some(job) = job {
                let result = run(&s, &job);
                update(&s, &job.id, |j| {
                    j.status = if j.cancel || s.shutdown.load(Ordering::SeqCst) {
                        "cancelled"
                    } else if result.is_ok() {
                        j.progress = 100.;
                        "completed"
                    } else {
                        "failed"
                    }
                    .into();
                    if let Err(e) = result {
                        j.logs.push(e)
                    }
                });
                s.history()
            } else {
                thread::sleep(Duration::from_millis(80))
            }
        }
    });
}
fn run(s: &Shared, j: &Job) -> Result<()> {
    if std::path::Path::new(&j.output).exists()
        && !j.plans.last().is_some_and(|p| p.iter().any(|a| a == "-y"))
    {
        return Err("Le fichier de sortie existe déjà. Activez explicitement l’écrasement ou choisissez un autre nom.".into());
    }
    let run_dir = s.dir.join("runs").join(&j.id);
    std::fs::create_dir_all(&run_dir).map_err(err)?;
    for (idx, plan) in j.plans.iter().enumerate() {
        if stopped(s, &j.id) {
            return Err(
                "Encodage annulé. La sortie partielle peut être supprimée manuellement.".into(),
            );
        }
        update(s, &j.id, |j| j.pass = idx + 1);
        let mut args = vec![
            "-nostdin".to_string(),
            "-progress".into(),
            "pipe:1".into(),
            "-nostats".into(),
        ];
        if !plan.iter().any(|a| a == "-y" || a == "-n") {
            args.push("-n".into())
        }
        args.extend(plan.clone());
        let mut child = command(std::path::Path::new(&j.binary))
            .args(&args)
            .current_dir(&run_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(err)?;
        let mut readers = vec![];
        if let Some(stdout) = child.stdout.take() {
            let s = s.clone();
            let id = j.id.clone();
            readers.push(thread::spawn(move || {
                for l in BufReader::new(stdout)
                    .lines()
                    .map_while(std::result::Result::ok)
                {
                    log(&s, &id, l)
                }
            }));
        }
        if let Some(stderr) = child.stderr.take() {
            let s = s.clone();
            let id = j.id.clone();
            readers.push(thread::spawn(move || {
                for l in BufReader::new(stderr)
                    .lines()
                    .map_while(std::result::Result::ok)
                {
                    log(&s, &id, l)
                }
            }));
        }
        let status = loop {
            if stopped(s, &j.id) {
                let _ = child.kill();
                break child.wait().map_err(err)?;
            }
            if let Some(status) = child.try_wait().map_err(err)? {
                break status;
            }
            thread::sleep(Duration::from_millis(50))
        };
        for reader in readers {
            let _ = reader.join();
        }
        if !status.success() {
            return Err(format!(
                "FFmpeg s’est arrêté avec le code {:?}.",
                status.code()
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_same_io() {
        assert!(validate(
            &[vec!["-i".into(), "a.mp4".into(), "a.mp4".into()]],
            "a.mp4"
        )
        .is_err());
    }
    #[test]
    fn accepts_literal_shell_characters() {
        assert!(validate(
            &[vec![
                "-i".into(),
                "C:/a & b.mp4".into(),
                "C:/out.mp4".into()
            ]],
            "C:/out.mp4"
        )
        .is_ok());
    }
    #[test]
    fn rejects_mismatched_output() {
        assert!(validate(&[vec!["-i".into(), "in".into(), "out".into()]], "other").is_err());
    }
}
