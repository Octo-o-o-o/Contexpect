//! Bounded invocation of explicitly pinned local tools. This is not a sandbox.
use ctxpect_schema::{Value, sha256_hex};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

static SERIAL: AtomicU64 = AtomicU64::new(0);
pub const LIMIT: u64 = 8 * 1024 * 1024;

pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new() -> io::Result<Self> {
        let root = std::env::temp_dir().join(format!(
            "ctxpect-tool-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&root)?;
        Ok(Self(root))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn private_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

pub fn read_small(path: &Path) -> io::Result<Vec<u8>> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::other("regular file required"));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(io::Error::other("input exceeds limit"));
    }
    Ok(bytes)
}

pub fn pinned_tool(profile: &Value, name: &str) -> Result<PathBuf, String> {
    let tool = profile.get(name).ok_or("tool profile missing")?;
    let path = PathBuf::from(
        tool.get("path")
            .and_then(Value::as_str)
            .ok_or("tool path missing")?,
    );
    if !path.is_absolute() {
        return Err("tool path must be absolute".into());
    }
    let digest = tool
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or("tool digest missing")?;
    // Binaries can exceed the protocol's document size limit.
    let mut input = fs::File::open(&path).map_err(|_| "tool unavailable")?;
    let mut hash = ctxpect_schema::Hasher::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = input.read(&mut buffer).map_err(|_| "tool unreadable")?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let actual = hash.finish();
    if actual != digest {
        return Err("tool digest drift".into());
    }
    Ok(path)
}

pub struct ToolOutput {
    pub code: Option<i32>,
    pub stdout: Vec<u8>,
    pub timed_out: bool,
    pub stdout_digest: String,
}

pub fn run(
    tool: &Path,
    args: &[String],
    input: &[u8],
    cwd: &Path,
    timeout: Duration,
) -> io::Result<ToolOutput> {
    run_with_env(tool, args, input, cwd, timeout, &[])
}

pub fn run_with_env(
    tool: &Path,
    args: &[String],
    input: &[u8],
    cwd: &Path,
    timeout: Duration,
    environment: &[(String, String)],
) -> io::Result<ToolOutput> {
    if input.len() as u64 > LIMIT {
        return Err(io::Error::other("input exceeds limit"));
    }
    let mut command = Command::new(tool);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (key, value) in environment {
        command.env(key, value);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = OwnedChild(command.spawn()?);
    let mut stdin = child
        .0
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("stdin unavailable"))?;
    let input = input.to_vec();
    std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let stdout = child
        .0
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("stdout unavailable"))?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = sender.send(result);
    });
    let started = Instant::now();
    let mut timed_out = false;
    let mut captured = None;
    let status = loop {
        if captured.is_none()
            && let Ok(result) = receiver.try_recv()
        {
            captured = Some(result?);
        }
        if captured.as_ref().is_some_and(|v| v.len() as u64 > LIMIT) {
            return Err(io::Error::other("output exceeds limit"));
        }
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            timed_out = true;
            child.terminate();
            break child.0.wait()?;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    child.terminate();
    let stdout = match captured {
        Some(bytes) => bytes,
        None => receiver
            .recv_timeout(Duration::from_millis(500))
            .map_err(|_| io::Error::other("output did not close"))??,
    };
    if stdout.len() as u64 > LIMIT {
        return Err(io::Error::other("output exceeds limit"));
    }
    Ok(ToolOutput {
        code: status.code(),
        stdout_digest: sha256_hex(&stdout),
        stdout,
        timed_out,
    })
}

struct OwnedChild(std::process::Child);
impl OwnedChild {
    fn terminate(&mut self) {
        #[cfg(unix)]
        {
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &format!("-{}", self.0.id())])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = self.0.kill();
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.terminate();
        let _ = self.0.wait();
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn stdin_closes_and_timeout_is_bounded() {
        let scratch = Scratch::new().unwrap();
        let output = run(
            Path::new("/bin/cat"),
            &[],
            b"sample input",
            &scratch.0,
            Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(output.code, Some(0));
        assert_eq!(output.stdout, b"sample input");
        assert!(!output.timed_out);
        let start = Instant::now();
        let output = run(
            Path::new("/bin/sh"),
            &["-c".into(), "sleep 5 & wait".into()],
            &[],
            &scratch.0,
            Duration::from_millis(50),
        )
        .unwrap();
        assert!(output.timed_out);
        assert!(start.elapsed() < Duration::from_secs(2));
    }
    #[test]
    fn symlink_document_is_refused_before_open() {
        let scratch = Scratch::new().unwrap();
        std::os::unix::fs::symlink("/dev/zero", scratch.0.join("input")).unwrap();
        assert!(read_small(&scratch.0.join("input")).is_err());
    }
}
