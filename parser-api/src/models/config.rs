use anyhow::Context;
use arc_swap::ArcSwap;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use rocket::serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};
use std::{fs, thread};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub admin_user: String,
    pub admin_api: String,
    pub tag_blacklist: Vec<String>,
    pub posts_domain: String,
    pub posts_limit: i32,
    pub rps_delay_ms: u64,
    pub max_retries: u64,
    pub group_weights: HashMap<String, f32>,
}

pub struct ConfigWatcher {
    pub stop: Arc<AtomicBool>,
    pub handle: Option<JoinHandle<()>>,
}
impl Drop for ConfigWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

pub fn load_config(p: &Path) -> anyhow::Result<Config> {
    let s = fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
    toml::from_str(&s).context("parsing config.toml")
}

pub fn default_path() -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from("config.toml"))
}

pub fn start_config_watcher(path: PathBuf) -> anyhow::Result<ConfigWatcher> {
    let parent = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();

    let handle = thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();

        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .expect("create watcher");

        watcher
            .watch(&parent, RecursiveMode::NonRecursive)
            .expect("watch parent");

        let mut last_mtime: Option<SystemTime> = file_mtime(&path).ok();

        while !stop_flag.load(Ordering::Relaxed) {
            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Ok(event)) => {
                    if event
                        .paths
                        .iter()
                        .any(|p| p == &path || p.file_name() == path.file_name())
                    {
                        thread::sleep(Duration::from_millis(120));

                        if let Ok(mtime) = file_mtime(&path) {
                            if last_mtime.is_none_or(|old| old < mtime) {
                                match reload_from(&path) {
                                    Ok(_) => {
                                        last_mtime = Some(mtime);
                                        eprintln!("[config] reloaded {}", path.display());
                                    }
                                    Err(e) => {
                                        eprintln!("[config] reload failed: {e:#}");
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("[config] watch error: {e}"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(ConfigWatcher {
        stop,
        handle: Some(handle),
    })
}

pub fn file_mtime(p: &Path) -> std::io::Result<SystemTime> {
    fs::metadata(p)?.modified()
}

pub static CONFIG: LazyLock<ArcSwap<Config>> = LazyLock::new(|| {
    let p = default_path().expect("config path");
    let cfg = load_config(&p).expect("initial config");
    ArcSwap::from_pointee(cfg)
});

pub fn cfg() -> Arc<Config> {
    CONFIG.load_full()
}

pub fn reload_from(p: &Path) -> anyhow::Result<()> {
    let new = load_config(p)?;
    let arc = Arc::new(new);
    CONFIG.store(arc.clone());
    eprintln!("[config] current value:\n{arc:#?}");
    Ok(())
}
