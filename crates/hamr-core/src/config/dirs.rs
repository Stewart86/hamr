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
    /// # Panics
    ///
    /// Panics if the system's project directories cannot be determined.
    #[must_use]
    pub fn new() -> Self {
        let project =
            ProjectDirs::from("", "", "hamr").expect("Failed to determine project directories");

        let config = project.config_dir().to_path_buf();
        let data = project.data_dir().to_path_buf();
        let cache = project.cache_dir().to_path_buf();

        Self {
            user_plugins: config.join("plugins"),
            config_file: config.join("config.json"),
            state_file: data.join("state.json"),
            colors_file: config.join("colors.json"),
            index_cache: config.join("plugin-indexes.json"),
            config,
            data,
            cache,
            builtin_plugins: Self::find_builtin_plugins(),
        }
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

    /// Find builtin plugins directory
    fn find_builtin_plugins() -> PathBuf {
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

        Self::system_plugins_path()
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

impl Default for Directories {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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
        let dirs = Directories::new();

        assert!(dirs.config.to_string_lossy().contains("hamr"));
        assert!(dirs.data.to_string_lossy().contains("hamr"));
        assert!(dirs.cache.to_string_lossy().contains("hamr"));
        assert!(dirs.config_file.to_string_lossy().ends_with("config.json"));
        assert!(dirs.state_file.to_string_lossy().ends_with("state.json"));
    }

    #[test]
    fn test_default_same_as_new() {
        let default_dirs = Directories::default();
        let new_dirs = Directories::new();

        assert_eq!(default_dirs.config, new_dirs.config);
        assert_eq!(default_dirs.data, new_dirs.data);
        assert_eq!(default_dirs.config_file, new_dirs.config_file);
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
        let temp_dir = tempfile::tempdir().unwrap();
        let plugins_dir = temp_dir.path().join("plugins");
        fs::create_dir_all(&plugins_dir).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let found = Directories::find_builtin_plugins();
        assert!(found.to_string_lossy().contains("plugins"));

        std::env::set_current_dir(original_dir).unwrap();
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
