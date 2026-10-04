//! `cargo xtask db start | stop | reset | url | run`: a throwaway PostgreSQL for the repository
//! tests (ADR-0003, ADR-0023, ADR-0024; architecture §11 "Database tests").
//!
//! The cluster lives in `.tools/pg-data/`, listens on 127.0.0.1 only and only on the port
//! `postgres` from `tools/dev/ports.json`. Its random password is written to `.tools/pg-password`
//! (ignored by Git, owner-only on Unix) and never appears anywhere else.

use std::fmt::Write as _;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context as _, bail};

use crate::bundle;
use crate::checks::arch_db_parity::{self, DATABASES, RECORD_PATH};
use crate::dev::Ports;
use crate::output;
use crate::repo::Repo;
use crate::tools::{self, TOOLS_DIR, Tool};

/// The harness reads these (crates/bb-testing); a test keeps both sides in step.
pub(crate) const DATABASES_VARIABLE: &str = "BB_TEST_DATABASES";
pub(crate) const POSTGRES_URL_VARIABLE: &str = "BB_TEST_POSTGRES_URL";
pub(crate) const RECORD_VARIABLE: &str = "BB_DB_PARITY_FILE";

/// The tool entry (tools/dev/tools.toml) that provides PostgreSQL.
const TOOL: &str = "postgresql";
/// The key in tools/dev/ports.json.
const PORT_KEY: &str = "postgres";
const USER: &str = "bugaboo";
const HOST: &str = "127.0.0.1";
const DATA_DIR: &str = "pg-data";
const PASSWORD_FILE: &str = "pg-password";
const LOG_FILE: &str = "pg-data.log";
/// Seconds `pg_ctl` waits for the server to start or stop.
const WAIT: &str = "90";
/// How long the server gets to start accepting connections.
const START_TIMEOUT: Duration = Duration::from_mins(1);
const POLL: Duration = Duration::from_millis(200);
/// Bytes of randomness in the password (hex-encoded, so twice as many characters).
const PASSWORD_BYTES: usize = 24;

/// What `cargo xtask db` was asked to do.
pub(crate) enum Action {
    Start,
    Stop,
    Reset,
    Url,
    /// Run a command with the test databases switched on.
    Run(Vec<String>),
}

/// Runs the action; the result says whether everything went well (for `Run`: the command too).
pub(crate) fn run(repo: &Repo, action: &Action) -> anyhow::Result<bool> {
    let cluster = Cluster::locate(repo)?;
    match action {
        Action::Start => {
            cluster.start()?;
            output::info(&format!(
                "PostgreSQL is running on {HOST}:{}.",
                cluster.port
            ));
            output::info("Print the connection URL with `cargo xtask db url`.");
        }
        Action::Stop => {
            if cluster.stop()? {
                output::info("PostgreSQL is stopped.");
            } else {
                output::info("PostgreSQL was not running.");
            }
        }
        Action::Reset => {
            cluster.stop()?;
            cluster.delete()?;
            cluster.start()?;
            output::info(&format!(
                "PostgreSQL was recreated and is running on {HOST}:{}.",
                cluster.port
            ));
        }
        Action::Url => output::line(&cluster.url()?),
        Action::Run(command) => return cluster.run_command(repo.root(), command),
    }
    Ok(true)
}

struct Cluster {
    root: PathBuf,
    tool: Tool,
    port: u16,
}

impl Cluster {
    fn locate(repo: &Repo) -> anyhow::Result<Self> {
        let tool = tools::load(repo)?
            .into_iter()
            .find(|tool| tool.name == TOOL)
            .with_context(|| format!("{} has no `{TOOL}` entry", tools::TOOLS_PATH))?;
        let port = Ports::load_port(repo.root(), PORT_KEY)?;
        Ok(Self {
            root: repo.root().to_path_buf(),
            tool,
            port,
        })
    }

    fn data(&self) -> PathBuf {
        self.root.join(TOOLS_DIR).join(DATA_DIR)
    }

    fn password_file(&self) -> PathBuf {
        self.root.join(TOOLS_DIR).join(PASSWORD_FILE)
    }

    fn program(&self, name: &str) -> anyhow::Result<PathBuf> {
        bundle::find_program(&self.root, &self.tool, name).with_context(|| {
            format!("PostgreSQL (`{name}`) is not installed. Run `cargo xtask setup` first.")
        })
    }

    /// `pg_ctl status` succeeds (exit code 0) while the server runs.
    fn is_running(&self) -> anyhow::Result<bool> {
        if !self.data().join("PG_VERSION").is_file() {
            return Ok(false);
        }
        let status = Command::new(self.program("pg_ctl")?)
            .arg("status")
            .arg("-D")
            .arg(self.data())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("could not run pg_ctl")?;
        Ok(status.success())
    }

    fn start(&self) -> anyhow::Result<()> {
        if self.is_running()? {
            return Ok(());
        }
        ensure_port_free(self.port)?;
        if !self.data().join("PG_VERSION").is_file() {
            self.create()?;
        }
        let log = self.root.join(TOOLS_DIR).join(LOG_FILE);
        self.launch(&log)?;
        let deadline = Instant::now() + START_TIMEOUT;
        while Instant::now() < deadline {
            if self.accepts_connections()? {
                return Ok(());
            }
            sleep(POLL);
        }
        bail!(
            "PostgreSQL did not start within {} seconds. Its log ({}) ends with:
{}",
            START_TIMEOUT.as_secs(),
            log.display(),
            tail(&log, 15)
        );
    }

    /// Starts the server in the background; it keeps running after xtask ends. Its messages go
    /// to the log file.
    fn launch(&self, log: &Path) -> anyhow::Result<()> {
        let pg_ctl = self.program("pg_ctl")?;
        let options = server_options(self.port);
        let status = if cfg!(windows) {
            // The server must not inherit our output pipes, or whoever reads them (a terminal
            // wrapper, CI) would wait for it to end. Windows passes every inheritable handle to
            // a started program; PowerShell's Start-Process does not.
            let arguments = format!(
                "start -D \"{}\" -l \"{}\" -o \"{options}\"",
                self.data().display(),
                log.display()
            );
            Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg(format!(
                    "Start-Process -FilePath '{}' -ArgumentList '{}' -WindowStyle Hidden",
                    pg_ctl.display().to_string().replace('\'', "''"),
                    arguments.replace('\'', "''")
                ))
                .stdin(Stdio::null())
                .status()
                .context("could not run powershell")?
        } else {
            Command::new(&pg_ctl)
                .args(["start", "-D"])
                .arg(self.data())
                .arg("-l")
                .arg(log)
                .arg("-o")
                .arg(options)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .context("could not run pg_ctl")?
        };
        if !status.success() {
            bail!("could not start pg_ctl for PostgreSQL.");
        }
        Ok(())
    }

    /// `pg_isready` succeeds (exit code 0) once the server accepts connections.
    fn accepts_connections(&self) -> anyhow::Result<bool> {
        let status = Command::new(self.program("pg_isready")?)
            .args(["-h", HOST, "-p", &self.port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .context("could not run pg_isready")?;
        Ok(status.success())
    }

    /// Makes a fresh cluster with a new random password.
    fn create(&self) -> anyhow::Result<()> {
        let password = random_password()?;
        write_private(&self.password_file(), &password)?;
        let data = self.data();
        if data.exists() {
            fs::remove_dir_all(&data)?;
        }
        let out = Command::new(self.program("initdb")?)
            .arg("-D")
            .arg(&data)
            .args([
                "-U",
                USER,
                "-E",
                "UTF8",
                "--no-locale",
                "--auth=scram-sha-256",
            ])
            .arg(format!("--pwfile={}", self.password_file().display()))
            .stdin(Stdio::null())
            .output()
            .context("could not run initdb")?;
        if !out.status.success() {
            bail!(
                "initdb failed:\n{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Ok(())
    }

    /// Stops the server; returns whether it was running.
    fn stop(&self) -> anyhow::Result<bool> {
        if !self.is_running()? {
            return Ok(false);
        }
        let status = Command::new(self.program("pg_ctl")?)
            .args(["stop", "-m", "fast", "-w", "-t", WAIT, "-D"])
            .arg(self.data())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .status()
            .context("could not run pg_ctl")?;
        if !status.success() {
            bail!("PostgreSQL did not stop; see its log in {TOOLS_DIR}/{LOG_FILE}.");
        }
        Ok(true)
    }

    /// Deletes the data and the password (the next start makes new ones).
    fn delete(&self) -> anyhow::Result<()> {
        if self.data().exists() {
            fs::remove_dir_all(self.data())?;
        }
        if self.password_file().exists() {
            fs::remove_file(self.password_file())?;
        }
        Ok(())
    }

    /// The URL of the maintenance database `postgres`; the test harness creates its own
    /// databases from there.
    fn url(&self) -> anyhow::Result<String> {
        let password = fs::read_to_string(self.password_file()).with_context(|| {
            format!(
                "There is no test database yet. Run `cargo xtask db start` first ({}).",
                self.password_file().display()
            )
        })?;
        Ok(connection_url(password.trim(), self.port))
    }

    /// `cargo xtask db run -- <command>`: starts the cluster when needed, runs the command with
    /// the test variables set, stops the cluster again if it was started here, then checks the
    /// parity record.
    fn run_command(&self, root: &Path, command: &[String]) -> anyhow::Result<bool> {
        let Some((program, args)) = command.split_first() else {
            bail!("Give a command after `--`, for example `cargo xtask db run -- cargo test`.");
        };
        let was_running = self.is_running()?;
        self.start()?;
        let record = root.join(RECORD_PATH);
        if let Some(folder) = record.parent() {
            fs::create_dir_all(folder)?;
        }
        let _ = fs::remove_file(&record);
        let result = Command::new(program)
            .args(args)
            .current_dir(root)
            .env(DATABASES_VARIABLE, DATABASES.join(","))
            .env(POSTGRES_URL_VARIABLE, self.url()?)
            .env(RECORD_VARIABLE, &record)
            .status()
            .with_context(|| format!("could not start `{program}`"));
        if !was_running {
            self.stop()?;
        }
        if !result?.success() {
            return Ok(false);
        }
        let text = fs::read_to_string(&record).unwrap_or_default();
        let problems = arch_db_parity::problems(&text);
        for problem in &problems {
            output::info(&format!("ARCH-DB-PARITY: {problem}"));
        }
        Ok(problems.is_empty())
    }
}

/// `postgres://bugaboo:<password>@127.0.0.1:<port>/postgres`
fn connection_url(password: &str, port: u16) -> String {
    format!("postgres://{USER}:{password}@{HOST}:{port}/postgres")
}

/// The server settings: loopback only, this port, no Unix socket, and no waiting for the disk
/// (the data is thrown away anyway).
fn server_options(port: u16) -> String {
    format!(
        "-p {port} -c listen_addresses={HOST} -c unix_socket_directories= -c fsync=off \
         -c synchronous_commit=off -c full_page_writes=off"
    )
}

/// Fails with a clear message when something else already uses the port (strictPort, ADR-0023).
fn ensure_port_free(port: u16) -> anyhow::Result<()> {
    match TcpListener::bind((HOST, port)) {
        Ok(_) => Ok(()),
        Err(error) => bail!(
            "Port {port} ({PORT_KEY} in tools/dev/ports.json) is busy: {error}. Stop what uses \
             it, or run `cargo xtask db stop` if it is an old test database. Bugaboo never picks \
             another port."
        ),
    }
}

fn random_password() -> anyhow::Result<String> {
    let mut bytes = [0_u8; PASSWORD_BYTES];
    getrandom::fill(&mut bytes).map_err(|error| anyhow::anyhow!("no random numbers: {error}"))?;
    let mut password = String::with_capacity(PASSWORD_BYTES * 2);
    for byte in bytes {
        let _ = write!(password, "{byte:02x}");
    }
    Ok(password)
}

/// Writes a secret file that only its owner can read (on Unix).
fn write_private(path: &Path, text: &str) -> anyhow::Result<()> {
    use std::io::Write as _;
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("could not write {}", path.display()))?;
    file.write_all(text.as_bytes())?;
    Ok(())
}

/// The last lines of a file, for error messages.
fn tail(path: &Path, lines: usize) -> String {
    let text = fs::read_to_string(path).unwrap_or_default();
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn the_url_points_at_loopback_and_the_port() {
        assert_eq!(
            connection_url("secret", 4711),
            "postgres://bugaboo:secret@127.0.0.1:4711/postgres"
        );
    }

    #[test]
    fn the_server_listens_on_loopback_only() {
        let options = server_options(4711);
        assert!(options.contains("-p 4711"));
        assert!(options.contains("listen_addresses=127.0.0.1"));
        assert!(!options.contains('*') && !options.contains("0.0.0.0"));
    }

    #[test]
    fn passwords_are_long_random_and_different() {
        let first = random_password().unwrap();
        assert_eq!(first.len(), PASSWORD_BYTES * 2);
        assert_ne!(first, random_password().unwrap());
    }

    #[test]
    fn bad_example_a_busy_port_fails_and_names_the_ports_file() {
        let held = TcpListener::bind((HOST, 0)).unwrap();
        let port = held.local_addr().unwrap().port();
        let error = ensure_port_free(port).unwrap_err().to_string();
        assert!(error.contains("tools/dev/ports.json"), "{error}");
        drop(held);
        ensure_port_free(port).unwrap();
    }

    #[test]
    fn the_secret_file_holds_exactly_the_text() {
        let temp = TempDir::new("db-private");
        let path = temp.path().join("nested/pw");
        write_private(&path, "abc").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "abc");
        write_private(&path, "z").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "z");
    }

    #[test]
    fn the_harness_reads_the_same_variable_names_and_database_names() {
        let source = include_str!("../../crates/bb-testing/src/lib.rs");
        for name in [DATABASES_VARIABLE, POSTGRES_URL_VARIABLE, RECORD_VARIABLE] {
            assert!(source.contains(&format!("\"{name}\"")), "{name} is missing");
        }
        for database in DATABASES {
            assert!(source.contains(&format!("\"{database}\"")), "{database}");
        }
    }

    #[test]
    fn a_missing_command_is_explained() {
        let repo = Repo::discover().unwrap();
        let cluster = Cluster::locate(&repo).unwrap();
        let error = cluster.run_command(repo.root(), &[]).unwrap_err();
        assert!(error.to_string().contains("after `--`"));
    }
}
