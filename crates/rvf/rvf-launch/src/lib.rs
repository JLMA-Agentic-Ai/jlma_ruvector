//! QEMU microVM launcher for RVF computational containers.
//!
//! This crate extracts a kernel image from an RVF file's KERNEL_SEG,
//! builds a QEMU command line, launches the VM, and provides a handle
//! for management (query, shutdown, kill) via QMP.

pub mod error;
pub mod extract;
pub mod qemu;
pub mod qmp;

use std::io::Read;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};

use rvf_types::kernel::KernelArch;

pub use error::LaunchError;

/// Configuration for launching an RVF microVM.
#[derive(Clone, Debug)]
pub struct LaunchConfig {
    /// Path to the RVF store file.
    pub rvf_path: PathBuf,
    /// Memory allocation in MiB.
    pub memory_mb: u32,
    /// Number of virtual CPUs.
    pub vcpus: u32,
    /// Host port to forward to the VM's API port (guest :8080).
    pub api_port: u16,
    /// Optional host port to forward to the VM's SSH port (guest :2222).
    pub ssh_port: Option<u16>,
    /// Whether to enable KVM acceleration (falls back to TCG if unavailable
    /// unless the kernel requires KVM).
    pub enable_kvm: bool,
    /// Override the QEMU binary path.
    pub qemu_binary: Option<PathBuf>,
    /// Extra arguments to pass to QEMU.
    pub extra_args: Vec<String>,
    /// Override the kernel image path (skip extraction from RVF).
    pub kernel_path: Option<PathBuf>,
    /// Override the initramfs path.
    pub initramfs_path: Option<PathBuf>,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            rvf_path: PathBuf::new(),
            memory_mb: 128,
            vcpus: 1,
            api_port: 8080,
            ssh_port: None,
            enable_kvm: true,
            qemu_binary: None,
            extra_args: Vec::new(),
            kernel_path: None,
            initramfs_path: None,
        }
    }
}

/// Current status of the microVM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VmStatus {
    /// QEMU process is running.
    Running,
    /// QEMU process has exited.
    Exited(Option<i32>),
}

/// A running QEMU microVM.
pub struct MicroVm {
    process: Child,
    api_port: u16,
    ssh_port: Option<u16>,
    qmp_socket: PathBuf,
    pid: u32,
    /// Holds the extracted kernel temp files alive.
    _extracted: Option<extract::ExtractedKernel>,
    /// Holds the work directory alive.
    _workdir: tempfile::TempDir,
}

/// Top-level launcher API.
pub struct Launcher;

impl Launcher {
    /// Extract kernel from an RVF file and launch it in a QEMU microVM.
    pub fn launch(config: &LaunchConfig) -> Result<MicroVm, LaunchError> {
        if !config.rvf_path.exists() {
            return Err(LaunchError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("RVF file not found: {}", config.rvf_path.display()),
            )));
        }

        // Extract kernel from RVF
        let extracted = extract::extract_kernel(&config.rvf_path)?;

        // Create a working directory for QMP socket, logs, etc.
        let workdir = tempfile::tempdir().map_err(LaunchError::TempFile)?;

        // Build the QEMU command
        let qemu_cmd = qemu::build_command(config, &extracted, workdir.path())?;

        let qmp_socket = qemu_cmd.qmp_socket.clone();

        // Spawn QEMU
        let mut command = qemu_cmd.command;
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = command.spawn().map_err(LaunchError::QemuSpawn)?;

        let pid = child.id();

        Ok(MicroVm {
            process: child,
            api_port: config.api_port,
            ssh_port: config.ssh_port,
            qmp_socket,
            pid,
            _extracted: Some(extracted),
            _workdir: workdir,
        })
    }

    /// Find the QEMU binary for the given architecture.
    pub fn find_qemu(arch: KernelArch) -> Result<PathBuf, LaunchError> {
        qemu::find_qemu(arch)
    }

    /// Check if KVM is available on this host.
    pub fn kvm_available() -> bool {
        qemu::kvm_available()
    }
}

impl MicroVm {
    /// Wait for the VM's API port to accept TCP connections.
    pub fn wait_ready(&mut self, timeout: Duration) -> Result<(), LaunchError> {
        let start = Instant::now();
        let addr = format!("127.0.0.1:{}", self.api_port);

        loop {
            // Check if the process has exited
            if let Some(exit) = self.try_wait_process()? {
                let mut stderr_buf = String::new();
                if let Some(ref mut stderr) = self.process.stderr {
                    let _ = stderr.read_to_string(&mut stderr_buf);
                }
                return Err(LaunchError::QemuExited {
                    code: exit,
                    stderr: stderr_buf,
                });
            }

            // Try connecting to the API port
            if TcpStream::connect_timeout(
                &addr.parse().unwrap(),
                Duration::from_millis(200),
            )
            .is_ok()
            {
                return Ok(());
            }

            if start.elapsed() >= timeout {
                return Err(LaunchError::Timeout {
                    seconds: timeout.as_secs(),
                });
            }

            std::thread::sleep(Duration::from_millis(250));
        }
    }

    /// Send a vector query to the running VM's HTTP API.
    pub fn query(
        &self,
        vector: &[f32],
        k: usize,
    ) -> Result<Vec<rvf_runtime::SearchResult>, LaunchError> {
        let _url = format!("http://127.0.0.1:{}/query", self.api_port);

        // Build JSON payload
        let payload = serde_json::json!({
            "vector": vector,
            "k": k,
        });
        let body = serde_json::to_vec(&payload)
            .map_err(|e| LaunchError::Io(std::io::Error::other(e)))?;

        // Use a raw TCP connection to send an HTTP POST (avoids depending
        // on a full HTTP client library).
        let addr = format!("127.0.0.1:{}", self.api_port);
        let mut stream = TcpStream::connect_timeout(
            &addr.parse().unwrap(),
            Duration::from_secs(5),
        )
        .map_err(LaunchError::Io)?;

        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(LaunchError::Io)?;

        use std::io::Write;
        let request = format!(
            "POST /query HTTP/1.1\r\n\
             Host: 127.0.0.1:{}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n",
            self.api_port,
            body.len(),
        );
        stream.write_all(request.as_bytes()).map_err(LaunchError::Io)?;
        stream.write_all(&body).map_err(LaunchError::Io)?;

        let mut response = String::new();
        stream.read_to_string(&mut response).map_err(LaunchError::Io)?;

        // Parse the HTTP response body (skip headers)
        let body_start = response
            .find("\r\n\r\n")
            .map(|i| i + 4)
            .unwrap_or(0);
        let resp_body = &response[body_start..];

        #[derive(serde::Deserialize)]
        struct QueryResult {
            id: u64,
            distance: f32,
        }

        let results: Vec<QueryResult> = serde_json::from_str(resp_body)
            .map_err(|e| LaunchError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;

        Ok(results
            .into_iter()
            .map(|r| rvf_runtime::SearchResult {
                id: r.id,
                distance: r.distance,
            })
            .collect())
    }

    /// Get the current VM status.
    pub fn status(&mut self) -> VmStatus {
        match self.process.try_wait() {
            Ok(Some(status)) => VmStatus::Exited(status.code()),
            Ok(None) => VmStatus::Running,
            Err(_) => VmStatus::Exited(None),
        }
    }

    /// Graceful shutdown: try QMP `system_powerdown`, fall back to SIGTERM.
    pub fn shutdown(&mut self) -> Result<(), LaunchError> {
        // Try QMP first
        if self.qmp_socket.exists() {
            match qmp::QmpClient::connect(&self.qmp_socket, Duration::from_secs(5)) {
                Ok(mut client) => {
                    let _ = client.system_powerdown();

                    // Wait up to 10 seconds for the VM to shut down
                    let start = Instant::now();
                    while start.elapsed() < Duration::from_secs(10) {
                        if let Ok(Some(_)) = self.process.try_wait() {
                            return Ok(());
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }

                    // Still running, try quit
                    let _ = client.quit();
                    let start = Instant::now();
                    while start.elapsed() < Duration::from_secs(5) {
                        if let Ok(Some(_)) = self.process.try_wait() {
                            return Ok(());
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
                Err(_) => {
                    // QMP not available, fall through to SIGTERM
                }
            }
        }

        // Fall back to SIGTERM (via kill on Unix)
        #[cfg(unix)]
        {
            unsafe {
                libc_kill(self.pid as i32);
            }
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(5) {
                if let Ok(Some(_)) = self.process.try_wait() {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }

        // Last resort: kill -9
        let _ = self.process.kill();
        let _ = self.process.wait();
        Ok(())
    }

    /// Force-kill the VM process immediately.
    pub fn kill(&mut self) -> Result<(), LaunchError> {
        self.process.kill().map_err(LaunchError::Io)?;
        let _ = self.process.wait();
        Ok(())
    }

    /// Get the QEMU process PID.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Get the API port.
    pub fn api_port(&self) -> u16 {
        self.api_port
    }

    /// Get the SSH port, if configured.
    pub fn ssh_port(&self) -> Option<u16> {
        self.ssh_port
    }

    /// Get the QMP socket path.
    pub fn qmp_socket(&self) -> &PathBuf {
        &self.qmp_socket
    }

    fn try_wait_process(&mut self) -> Result<Option<Option<i32>>, LaunchError> {
        match self.process.try_wait() {
            Ok(Some(status)) => Ok(Some(status.code())),
            Ok(None) => Ok(None),
            Err(e) => Err(LaunchError::Io(e)),
        }
    }
}

impl Drop for MicroVm {
    fn drop(&mut self) {
        // Best-effort cleanup: try to kill the process if still running.
        if let Ok(None) = self.process.try_wait() {
            let _ = self.process.kill();
            let _ = self.process.wait();
        }
    }
}

/// Send SIGTERM on Unix. Avoids a libc dependency by using a raw syscall.
#[cfg(unix)]
unsafe fn libc_kill(pid: i32) {
    // SIGTERM = 15 on all Unix platforms
    // We use std::process::Command as a portable way to send signals.
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .output();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = LaunchConfig::default();
        assert_eq!(config.memory_mb, 128);
        assert_eq!(config.vcpus, 1);
        assert_eq!(config.api_port, 8080);
        assert!(config.enable_kvm);
    }

    #[test]
    fn vm_status_variants() {
        assert_eq!(VmStatus::Running, VmStatus::Running);
        assert_eq!(VmStatus::Exited(Some(0)), VmStatus::Exited(Some(0)));
        assert_ne!(VmStatus::Running, VmStatus::Exited(None));
    }
}
