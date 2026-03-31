use directories::ProjectDirs;
use std::path::PathBuf;

/// Application directories following XDG spec
#[derive(Debug, Clone)]
pub struct Directories {
    /// Config directory (~/.config/hamr)
    pub config: PathBuf,

    /// Data directory (~/.local/share/hamr)
    pub data: PathBuf,

    /// Cache directory (~/.cache/hamr)
    pub cache: PathBuf,

    /// User plugins directory (~/.config/hamr/plugins)
    pub user_plugins: PathBuf,

    /// Builtin plugins directory (next to binary or in repo)
    pub builtin_plugins: PathBuf,

    /// Index cache file
    pub index_cache: PathBuf,

    /// Config file path
    pub config_file: PathBuf,

    /// Persistent state file
    pub state_file: PathBuf,

    /// Colors/theme file
    pub colors_file: PathBuf,
}

impl Directories {
    /// Create a new `Directories` instance with standard XDG paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the system's project directories cannot be determined.
    pub fn new() -> crate::Result<Self> {
        let project = ProjectDirs::from("", "", "hamr").ok_or_else(|| {
            crate::Error::Config("Failed to determine project directories".into())
        })?;

        let config = project.config_dir().to_path_buf();
        let data = project.data_dir().to_path_buf();
        let cache = project.cache_dir().to_path_buf();

        Ok(Self {
            user_plugins: config.join("plugins"),
            config_file: config.join("config.json"),
            state_file: data.join("state.json"),
            colors_file: config.join("colors.json"),
            index_cache: config.join("plugin-indexes.json"),
            config,
            data,
            cache,
            builtin_plugins: Self::find_builtin_plugins(),
        })
    }

    #[must_use]
    pub fn with_base(base: PathBuf) -> Self {
        Self {
            user_plugins: base.join("plugins"),
            config_file: base.join("config.json"),
            state_file: base.join("state.json"),
            colors_file: base.join("colors.json"),
            index_cache: base.join("plugin-indexes.json"),
            builtin_plugins: base.join("builtin-plugins"),
            config: base.clone(),
            data: base.clone(),
            cache: base,
        }
    }

    fn find_builtin_plugins() -> PathBuf {
        if let Some(path) = Self::packaged_plugins_path() {
            return path;
        }

        if let Ok(exe_path) = std::env::current_exe() {
            let exe_dir = exe_path.parent().unwrap_or(&exe_path);

            // Check next to the binary first (e.g. bin/plugins)
            let plugins_dir = exe_dir.join("plugins");
            if plugins_dir.exists() {
                return plugins_dir;
            }

            // Check FHS-style share path relative to binary (e.g. Nix: ../share/hamr/plugins)
            let share_plugins = exe_dir.join("../share/hamr/plugins");
            if share_plugins.exists() {
                return share_plugins.canonicalize().unwrap_or(share_plugins);
            }
        }

        // Dev paths - only used when running from source/dev builds
        let dev_paths: [PathBuf; 2] = [PathBuf::from("plugins"), PathBuf::from("../hamr/plugins")];

        for path in dev_paths {
            if path.exists() {
                return path.canonicalize().unwrap_or(path);
            }
        }

        if let Some(path) = Self::xdg_data_plugins_path() {
            return path;
        }

        Self::system_plugins_path()
    }

    fn packaged_plugins_path() -> Option<PathBuf> {
        let plugin_dir = std::env::var_os("HAMR_PLUGIN_DIR")?;
        let path = PathBuf::from(plugin_dir);
        path.exists().then(|| path.canonicalize().unwrap_or(path))
    }

    fn xdg_data_plugins_path() -> Option<PathBuf> {
        let xdg_data_dirs = std::env::var_os("XDG_DATA_DIRS")?;

        std::env::split_paths(&xdg_data_dirs)
            .map(|dir| dir.join("hamr/plugins"))
            .find(|path| path.exists())
            .map(|path| path.canonicalize().unwrap_or(path))
    }

    /// Get the system-wide plugins path for the current platform
    #[cfg(target_os = "macos")]
    fn system_plugins_path() -> PathBuf {
        PathBuf::from("/Library/Application Support/hamr/plugins")
    }

    /// Get the system-wide plugins path for the current platform
    #[cfg(not(target_os = "macos"))]
    fn system_plugins_path() -> PathBuf {
        PathBuf::from("/usr/share/hamr/plugins")
    }

    /// Ensure all directories exist.
    ///
    /// # Errors
    ///
    /// Returns an error if any directory cannot be created.
    pub fn ensure_exists(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config)?;
        std::fs::create_dir_all(&self.data)?;
        std::fs::create_dir_all(&self.cache)?;
        std::fs::create_dir_all(&self.user_plugins)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    fn test_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct ScopedCurrentDir {
        original: PathBuf,
    }

    impl ScopedCurrentDir {
        fn set(path: &std::path::Path) -> Self {
            let original = std::env::current_dir().unwrap();
            std::env::set_current_dir(path).unwrap();
            Self { original }
        }
    }

    impl Drop for ScopedCurrentDir {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original).unwrap();
        }
    }

    struct ScopedEnvVar {
        key: &'static str,
        original: Option<OsString>,
    }

    impl ScopedEnvVar {
        fn set(key: &'static str, value: &std::path::Path) -> Self {
            let original = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, original }
        }
    }

    impl Drop for ScopedEnvVar {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
        }
    }

    #[test]
    fn test_with_base_sets_all_paths() {
        let base = PathBuf::from("/tmp/test-hamr");
        let dirs = Directories::with_base(base.clone());

        assert_eq!(dirs.config, base);
        assert_eq!(dirs.data, base);
        assert_eq!(dirs.cache, base);
        assert_eq!(dirs.user_plugins, base.join("plugins"));
        assert_eq!(dirs.builtin_plugins, base.join("builtin-plugins"));
        assert_eq!(dirs.config_file, base.join("config.json"));
        assert_eq!(dirs.state_file, base.join("state.json"));
        assert_eq!(dirs.colors_file, base.join("colors.json"));
        assert_eq!(dirs.index_cache, base.join("plugin-indexes.json"));
    }

    #[test]
    fn test_ensure_exists_creates_directories() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path().join("hamr-test-subdir");
        let dirs = Directories::with_base(base.clone());

        assert!(!dirs.user_plugins.exists());

        dirs.ensure_exists().unwrap();

        assert!(dirs.config.exists());
        assert!(dirs.data.exists());
        assert!(dirs.cache.exists());
        assert!(dirs.user_plugins.exists());
    }

    #[test]
    fn test_ensure_exists_idempotent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dirs = Directories::with_base(temp_dir.path().to_path_buf());

        dirs.ensure_exists().unwrap();
        dirs.ensure_exists().unwrap();

        assert!(dirs.config.exists());
    }

    #[test]
    fn test_new_returns_valid_xdg_paths() {
        let dirs = Directories::new().unwrap();

        assert!(dirs.config.to_string_lossy().contains("hamr"));
        assert!(dirs.data.to_string_lossy().contains("hamr"));
        assert!(dirs.cache.to_string_lossy().contains("hamr"));
        assert!(dirs.config_file.to_string_lossy().ends_with("config.json"));
        assert!(dirs.state_file.to_string_lossy().ends_with("state.json"));
    }

    #[test]
    fn test_system_plugins_path() {
        let path = Directories::system_plugins_path();
        #[cfg(target_os = "macos")]
        assert_eq!(
            path,
            PathBuf::from("/Library/Application Support/hamr/plugins")
        );
        #[cfg(not(target_os = "macos"))]
        assert_eq!(path, PathBuf::from("/usr/share/hamr/plugins"));
    }

    #[test]
    fn test_find_builtin_plugins_uses_dev_paths() {
        let _lock = test_env_lock().lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        let _cwd = ScopedCurrentDir::set(temp_dir.path());

        let found = Directories::find_builtin_plugins();
        assert!(found.to_string_lossy().contains("plugins"));
    }

    #[test]
    fn test_find_builtin_plugins_uses_xdg_data_dirs() {
        let _lock = test_env_lock().lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let share_dir = temp_dir.path().join("share");
        let plugins_dir = share_dir.join("hamr/plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        let isolated_cwd = temp_dir.path().join("cwd");
        fs::create_dir_all(&isolated_cwd).unwrap();

        let _cwd = ScopedCurrentDir::set(&isolated_cwd);
        let _xdg = ScopedEnvVar::set("XDG_DATA_DIRS", &share_dir);

        let found = Directories::find_builtin_plugins();
        assert_eq!(found, plugins_dir.canonicalize().unwrap());
    }

    #[test]
    fn test_find_builtin_plugins_uses_packaged_plugin_dir_env() {
        let _lock = test_env_lock().lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let plugins_dir = temp_dir.path().join("hamr/plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        let isolated_cwd = temp_dir.path().join("cwd");
        fs::create_dir_all(&isolated_cwd).unwrap();

        let _cwd = ScopedCurrentDir::set(&isolated_cwd);
        let _plugin_dir = ScopedEnvVar::set("HAMR_PLUGIN_DIR", &plugins_dir);

        let found = Directories::find_builtin_plugins();
        assert_eq!(found, plugins_dir.canonicalize().unwrap());
    }

    #[test]
    fn test_directories_clone() {
        let dirs = Directories::with_base(PathBuf::from("/tmp/test"));
        let cloned = dirs.clone();
        assert_eq!(dirs.config, cloned.config);
        assert_eq!(dirs.data, cloned.data);
    }

    #[test]
    fn test_directories_debug() {
        let dirs = Directories::with_base(PathBuf::from("/tmp/test"));
        let debug = format!("{dirs:?}");
        assert!(debug.contains("Directories"));
        assert!(debug.contains("/tmp/test"));
    }
}
