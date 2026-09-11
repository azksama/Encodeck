use super::*;
use std::{thread, time::Instant};
fn core() -> Shared {
    let base = std::env::var("ENCODECK_TEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("encodeck-tests"));
    let dir = base.join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&dir).unwrap();
    Arc::new(Core {
        dir,
        config: Mutex::new(Config::default()),
        jobs: Mutex::new(vec![]),
        installing: Mutex::new(String::new()),
        version_lock: Mutex::new(()),
        shutdown: AtomicBool::new(false),
    })
}
fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}
fn wait(s: &Shared, id: &str) -> engine::Job {
    let start = Instant::now();
    loop {
        let job = s
            .jobs
            .lock()
            .unwrap()
            .iter()
            .find(|j| j.id == id)
            .unwrap()
            .clone();
        if !["queued", "running"].contains(&job.status.as_str()) {
            return job;
        }
        assert!(start.elapsed() < Duration::from_secs(30), "Job timed out");
        thread::sleep(Duration::from_millis(60));
    }
}
#[test]
#[ignore = "Downloads an actual FFmpeg build to an isolated test directory"]
fn native_end_to_end() {
    let s = core();
    let releases = versions::catalog().unwrap();
    let release = releases.first().expect("No available build");
    versions::install(s.clone(), release.tag.clone()).unwrap();
    let bin = s.binary().unwrap();
    assert!(bin.exists());
    let source = s.dir.join("source & café.mp4");
    let output = s.dir.join("output & café.mp4");
    let status = engine::command(&bin)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=320x240:r=25",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
        ])
        .arg(&source)
        .status()
        .unwrap();
    assert!(status.success());
    let info = engine::probe(&bin, source.to_str().unwrap()).unwrap();
    assert!(info["streams"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["codec_type"] == "video"));
    engine::start_worker(s.clone());
    let src = source.to_str().unwrap();
    let out = output.to_str().unwrap();
    let plan = strings(&[
        "-i",
        src,
        "-c:v",
        "libx264",
        "-preset",
        "ultrafast",
        "-c:a",
        "aac",
        out,
    ]);
    let id = engine::enqueue(&s, vec![plan.clone()], out.into(), 1.).unwrap();
    assert_eq!(wait(&s, &id).status, "completed");
    assert!(output.metadata().unwrap().len() > 1000);
    // Refusing overwrite must surface as failure, rather than hanging at a prompt.
    let id = engine::enqueue(&s, vec![plan], out.into(), 1.).unwrap();
    assert_eq!(wait(&s, &id).status, "failed");
    let two = s.dir.join("two-pass.mp4");
    let two = two.to_str().unwrap();
    let plans = vec![
        strings(&[
            "-i", src, "-c:v", "libx264", "-b:v", "500k", "-pass", "1", "-an", "-f", "null", "-",
        ]),
        strings(&[
            "-i", src, "-c:v", "libx264", "-b:v", "500k", "-pass", "2", "-c:a", "aac", two,
        ]),
    ];
    let id = engine::enqueue(&s, plans, two.into(), 1.).unwrap();
    let j = wait(&s, &id);
    assert_eq!(j.status, "completed", "{:?}", j.logs);
    let cancel = s.dir.join("cancel.mkv");
    let id = engine::enqueue(
        &s,
        vec![strings(&[
            "-re",
            "-stream_loop",
            "-1",
            "-i",
            src,
            "-c",
            "copy",
            cancel.to_str().unwrap(),
        ])],
        cancel.to_string_lossy().into(),
        0.,
    )
    .unwrap();
    thread::sleep(Duration::from_millis(500));
    {
        let mut js = s.jobs.lock().unwrap();
        js.iter_mut().find(|j| j.id == id).unwrap().cancel = true;
    }
    assert_eq!(wait(&s, &id).status, "cancelled");
    let missing = s.dir.join("missing.mp4");
    let id = engine::enqueue(
        &s,
        vec![strings(&[
            "-i",
            "/nonexistent-file-encodeck-test",
            "-c",
            "copy",
            missing.to_str().unwrap(),
        ])],
        missing.to_string_lossy().into(),
        0.,
    )
    .unwrap();
    assert_eq!(wait(&s, &id).status, "failed");
    s.history();
    let persisted: Vec<engine::Job> =
        serde_json::from_slice(&std::fs::read(s.dir.join("history.json")).unwrap()).unwrap();
    assert_eq!(persisted.len(), 5);
    assert!(versions::remove(&s, &release.tag).is_err());
    s.shutdown.store(true, Ordering::SeqCst);
    println!("Verified install, probe, transcode, two-pass, overwrite protection, cancellation, errors and history: {}",s.dir.display());
}
