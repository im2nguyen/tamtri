//! Environment preserved when spawning harness subprocesses.
//!
//! Claude Code reads OAuth credentials from the macOS Keychain only when `USER`
//! is set. `env_clear()` without it yields "Not logged in · Please run /login".

const SPAWN_ENV_KEYS: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "TMPDIR",
    "LANG",
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_HOME",
    "CODEX_HOME",
    "HERMES_HOME",
];

pub fn preserve_spawn_env(cmd: &mut std::process::Command) {
    for key in SPAWN_ENV_KEYS {
        preserve_env(cmd, key);
    }
}

pub fn preserve_spawn_env_tokio(cmd: &mut tokio::process::Command) {
    for key in SPAWN_ENV_KEYS {
        preserve_env_tokio(cmd, key);
    }
}

fn preserve_env(cmd: &mut std::process::Command, key: &str) {
    if let Ok(value) = std::env::var(key) {
        cmd.env(key, value);
    }
}

fn preserve_env_tokio(cmd: &mut tokio::process::Command, key: &str) {
    if let Ok(value) = std::env::var(key) {
        cmd.env(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn preserve_spawn_env_includes_user_when_set() {
        let _guard = ENV_LOCK.lock().expect("lock");
        let previous = std::env::var_os("USER");
        unsafe {
            std::env::set_var("USER", "tamtri-test-user");
        }
        let mut cmd = std::process::Command::new("echo");
        preserve_spawn_env(&mut cmd);
        let envs: Vec<_> = cmd.get_envs().collect();
        let has_user = envs
            .iter()
            .any(|(key, value)| key == &"USER" && value == &Some(std::ffi::OsStr::new("tamtri-test-user")));
        if let Some(value) = previous {
            unsafe {
                std::env::set_var("USER", value);
            }
        } else {
            unsafe {
                std::env::remove_var("USER");
            }
        }
        assert!(has_user);
    }
}
