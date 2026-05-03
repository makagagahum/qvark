use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "Qorx";
pub const LOCAL_BASE: &str = "http://127.0.0.1:47187";
pub const PORTABLE_MARKER: &str = "qorx.portable";
pub const PORTABLE_DATA_DIR: &str = "qorx-data";
pub const PORTABLE_EXE_MAX_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub portable: bool,
    pub stats_file: PathBuf,
    pub atom_file: PathBuf,
    pub index_file: PathBuf,
    pub context_protobuf_file: PathBuf,
    pub response_cache_file: PathBuf,
    pub integration_report_file: PathBuf,
    pub provenance_file: PathBuf,
    pub security_keys_file: PathBuf,
    pub shim_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn qorx_home_wins_over_portable_marker() {
        let selected = super::choose_data_dir(
            PathBuf::from(r"C:\portable"),
            Some(PathBuf::from(r"D:\QorxHome")),
            true,
            true,
            PathBuf::from(r"C:\Users\Example\AppData\Local\qorx"),
        );

        assert_eq!(selected, PathBuf::from(r"D:\QorxHome"));
    }

    #[test]
    fn portable_marker_keeps_data_next_to_exe() {
        let selected = super::choose_data_dir(
            PathBuf::from(r"C:\Tools\Qorx"),
            None,
            false,
            true,
            PathBuf::from(r"C:\Users\Example\AppData\Local\qorx"),
        );

        assert_eq!(selected, PathBuf::from(r"C:\Tools\Qorx\qorx-data"));
    }

    #[test]
    fn normal_mode_uses_platform_data_dir() {
        let selected = super::choose_data_dir(
            PathBuf::from(r"C:\Tools\Qorx"),
            None,
            false,
            false,
            PathBuf::from(r"C:\Users\Example\AppData\Local\qorx"),
        );

        assert_eq!(
            selected,
            PathBuf::from(r"C:\Users\Example\AppData\Local\qorx")
        );
    }
}

impl AppPaths {
    pub fn resolve() -> Result<Self> {
        resolve_paths(true)
    }
}

fn resolve_paths(create_dirs: bool) -> Result<AppPaths> {
    let normal_data_dir = ProjectDirs::from("ai", "qorx", APP_NAME)
        .ok_or_else(|| anyhow!("could not resolve local app data directory"))?
        .data_local_dir()
        .to_path_buf();
    let exe_dir = current_exe_dir()?;
    let qorx_home = env::var_os("QORX_HOME").map(PathBuf::from);
    let portable_env = truthy_env("QORX_PORTABLE");
    let marker_exists = exe_dir.join(PORTABLE_MARKER).exists();
    let portable = qorx_home.is_none() && (portable_env || marker_exists);
    let data_dir = choose_data_dir(
        exe_dir,
        qorx_home,
        portable_env,
        marker_exists,
        normal_data_dir,
    );
    if create_dirs {
        fs::create_dir_all(&data_dir)?;
    }
    let shim_dir = data_dir.join("shims");
    if create_dirs {
        fs::create_dir_all(&shim_dir)?;
    }
    Ok(AppPaths {
        data_dir: data_dir.clone(),
        portable,
        stats_file: data_dir.join("stats.pb"),
        atom_file: data_dir.join("quarks.pb"),
        index_file: data_dir.join("repo_index.pb"),
        context_protobuf_file: data_dir.join("qorx-context.pb"),
        response_cache_file: data_dir.join("response_cache.pb"),
        integration_report_file: data_dir.join("integrations.pb"),
        provenance_file: data_dir.join("qorx-provenance.pb"),
        security_keys_file: data_dir.join("qorx-security-keys.pb"),
        shim_dir,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableReport {
    pub portable: bool,
    pub exe_path: String,
    pub exe_size_bytes: u64,
    pub max_portable_exe_bytes: u64,
    pub exe_within_size_target: bool,
    pub data_dir: String,
    pub marker: String,
    pub marker_exists: bool,
    pub env_home: Option<String>,
    pub env_portable: bool,
    pub q_drive_hint: String,
    pub boundary: String,
}

pub fn portable_report(paths: &AppPaths) -> Result<PortableReport> {
    let exe_dir = current_exe_dir()?;
    let exe_path = env::current_exe()?;
    let exe_size_bytes = fs::metadata(&exe_path).map(|meta| meta.len()).unwrap_or(0);
    let marker = exe_dir.join(PORTABLE_MARKER);
    Ok(PortableReport {
        portable: paths.portable,
        exe_path: exe_path.display().to_string(),
        exe_size_bytes,
        max_portable_exe_bytes: PORTABLE_EXE_MAX_BYTES,
        exe_within_size_target: exe_size_bytes > 0 && exe_size_bytes <= PORTABLE_EXE_MAX_BYTES,
        data_dir: paths.data_dir.display().to_string(),
        marker: marker.display().to_string(),
        marker_exists: marker.exists(),
        env_home: env::var("QORX_HOME").ok(),
        env_portable: truthy_env("QORX_PORTABLE"),
        q_drive_hint: format!(
            "Community Edition stores local data at \"{}\". Managed drive-letter UX belongs to Qorx Local Pro.",
            paths.data_dir.display()
        ),
        boundary: "The Community Edition executable contains the Qorx language, index, cache, AIM reader, and provenance logic. Tray, daemon, provider routing, managed drive UX, and RAM-disk setup belong to Qorx Local Pro.".to_string(),
    })
}

pub fn init_portable() -> Result<PortableReport> {
    let exe_dir = current_exe_dir()?;
    fs::create_dir_all(&exe_dir)?;
    fs::write(
        exe_dir.join(PORTABLE_MARKER),
        "Qorx portable mode: keep index, quarks, cache, stats, and shims beside qorx.exe.\n",
    )?;
    let data_dir = exe_dir.join(PORTABLE_DATA_DIR);
    fs::create_dir_all(&data_dir)?;
    let shim_dir = data_dir.join("shims");
    fs::create_dir_all(&shim_dir)?;
    let paths = AppPaths {
        data_dir: data_dir.clone(),
        portable: true,
        stats_file: data_dir.join("stats.pb"),
        atom_file: data_dir.join("quarks.pb"),
        index_file: data_dir.join("repo_index.pb"),
        context_protobuf_file: data_dir.join("qorx-context.pb"),
        response_cache_file: data_dir.join("response_cache.pb"),
        integration_report_file: data_dir.join("integrations.pb"),
        provenance_file: data_dir.join("qorx-provenance.pb"),
        security_keys_file: data_dir.join("qorx-security-keys.pb"),
        shim_dir,
    };
    portable_report(&paths)
}

fn current_exe_dir() -> Result<PathBuf> {
    env::current_exe()?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("could not resolve qorx executable directory"))
}

fn truthy_env(key: &str) -> bool {
    env::var(key)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on" | "portable"
            )
        })
        .unwrap_or(false)
}

fn choose_data_dir(
    exe_dir: PathBuf,
    qorx_home: Option<PathBuf>,
    portable_env: bool,
    marker_exists: bool,
    normal_data_dir: PathBuf,
) -> PathBuf {
    if let Some(path) = qorx_home {
        return path;
    }
    if portable_env || marker_exists {
        return exe_dir.join(PORTABLE_DATA_DIR);
    }
    normal_data_dir
}
