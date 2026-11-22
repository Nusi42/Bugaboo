//! The one list of steps of the try-out, run the same way on a clean Windows and in a fresh Linux
//! with systemd. Each step has its name in the results file, and its commands for each of the two
//! systems: PowerShell in Windows Sandbox, bash in WSL. A script is the facts of the run (the
//! port, the programs, the words `bugaboo service status` says), the helpers of its system and
//! the steps in this order; it writes one line per step to its results file and stops at the
//! first step that fails.

use std::fmt::Write as _;

use bb_core::data_folder::{BACKUPS_FOLDER, DATABASE_FILE, SETUP_CODE_FILE, VERSION_FILE};
use bb_core::message;
use bb_core::release::CHECKSUMS_FILE;
use bb_core::service::{RESTART_DELAY, SYSTEMD_UNIT, WINDOWS_NAME};

/// The system a script is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Guest {
    /// A clean Windows in Windows Sandbox, run by its administrator (`SYSTEM`).
    Windows,
    /// A fresh WSL distribution with systemd, run by root.
    Linux,
}

/// One step of the try-out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TryStep {
    /// Windows: Smart App Control, on in a clean Windows 11, blocks a program that carries no
    /// signature of a known publisher; while the release is not signed for Windows, the run notes
    /// that it is on and switches it off inside the sandbox. Skipped on Linux.
    AppControl,
    /// The program of the release is checked with `bugaboo update verify`, when the release
    /// folder is signed.
    Verify,
    /// The program of the previous release writes the settings (`setup --non-interactive`).
    PreviousSetup,
    /// The previous release starts once in the console and makes its database.
    PreviousData,
    /// The program of the release takes the place of the previous one.
    Replace,
    /// While every signed-in account may change the folder of the program, `bugaboo service
    /// install` refuses and installs nothing.
    RefuseOpen,
    /// The account of the service may no longer write into the folder of the backups, so the
    /// backup before the upgrade fails.
    BlockBackups,
    /// `bugaboo service install` with the program of the release; the service starts and stops
    /// at the upgrade.
    Install,
    /// The stopped upgrade is not started again: `service status` says so for longer than the
    /// service manager waits before a restart.
    NoRestart,
    /// The folder of the backups is given back and the service is started again.
    Restart,
    /// The server answers on its port.
    Answers,
    /// The backup before the upgrade is there, and the data names the new version.
    Upgraded,
    /// The setup code of the first administrator is in the data folder.
    SetupCode,
    /// `bugaboo service status` says that the service runs.
    Status,
    /// The process of the server is killed; the service manager starts it again.
    CrashRestart,
    /// `bugaboo service uninstall`; the data stays.
    Uninstall,
}

/// The steps, in the order they run.
pub(crate) const STEPS: [TryStep; 16] = [
    TryStep::AppControl,
    TryStep::Verify,
    TryStep::PreviousSetup,
    TryStep::PreviousData,
    TryStep::Replace,
    TryStep::RefuseOpen,
    TryStep::BlockBackups,
    TryStep::Install,
    TryStep::NoRestart,
    TryStep::Restart,
    TryStep::Answers,
    TryStep::Upgraded,
    TryStep::SetupCode,
    TryStep::Status,
    TryStep::CrashRestart,
    TryStep::Uninstall,
];

/// What the step `Upgraded` leaves in the results folder for the check on the host: the names in
/// the folder of the backups, and the version file of the data folder.
pub(crate) const UPGRADED_BACKUPS: &str = "upgraded-backups.txt";
pub(crate) const UPGRADED_VERSION: &str = "upgraded-version.txt";

/// The file of the results in the results folder, and the log of every command.
pub(crate) const RESULTS_FILE: &str = "results.txt";
pub(crate) const LOG_FILE: &str = "log.txt";
/// The last line of a run that went through.
pub(crate) const PASSED: &str = "passed";

impl TryStep {
    /// The name of the step in the results file.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::AppControl => "app-control",
            Self::Verify => "verify",
            Self::PreviousSetup => "previous-setup",
            Self::PreviousData => "previous-data",
            Self::Replace => "replace",
            Self::RefuseOpen => "refuse-open",
            Self::BlockBackups => "block-backups",
            Self::Install => "install",
            Self::NoRestart => "no-restart",
            Self::Restart => "restart",
            Self::Answers => "answers",
            Self::Upgraded => "upgraded",
            Self::SetupCode => "setup-code",
            Self::Status => "status",
            Self::CrashRestart => "crash-restart",
            Self::Uninstall => "uninstall",
        }
    }

    /// The commands of the step in PowerShell. They throw when the step fails, and set
    /// `$script:Skipped` to a reason when it does not apply.
    fn powershell(self) -> &'static str {
        match self {
            Self::AppControl => {
                r"$key = 'HKLM:\SYSTEM\CurrentControlSet\Control\CI\Policy'; if ((Get-ItemProperty $key -ErrorAction SilentlyContinue).VerifiedAndReputablePolicyState -eq 1) { Note 'note app-control: Smart App Control is on and blocks the unsigned programs; switched off in this sandbox'; Set-ItemProperty $key -Name VerifiedAndReputablePolicyState -Value 0; Run CiTool.exe --refresh --json } else { $script:Skipped = 'Smart App Control is off' }"
            }
            Self::Verify => {
                "if (Test-Path (Join-Path $Release $Checksums)) { Run $Next update verify $Next } else { $script:Skipped = 'the release folder is not signed' }"
            }
            Self::PreviousSetup => {
                "New-Item -ItemType Directory -Force $Install | Out-Null; Copy-Item $PreviousProgram $Program; Run $Program setup --non-interactive --config $Settings --data-dir $Data --port $Port; Must (Test-Path $Settings) 'setup wrote no settings file'"
            }
            Self::PreviousData => {
                "$p = Start-Process -FilePath $Program -ArgumentList ('serve --config \"' + $Settings + '\"') -NoNewWindow -PassThru -RedirectStandardOutput (Join-Path $Here 'previous-serve.txt') -RedirectStandardError (Join-Path $Here 'previous-serve-errors.txt'); try { Answers 180 } finally { if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }; $p.WaitForExit() }; Must (Test-Path (Join-Path $Data $DatabaseFile)) 'the previous release made no database'"
            }
            Self::Replace => "Copy-Item $Next $Program -Force",
            Self::RefuseOpen => {
                "Run icacls.exe $Install /grant \"${SignedIn}:(OI)(CI)M\"; $old = $ErrorActionPreference; $ErrorActionPreference = 'Continue'; $said = (& $Program service install --config $Settings 2>&1 | ForEach-Object { \"$_\" }) -join ' '; $code = $LASTEXITCODE; $ErrorActionPreference = $old; Add-Content -Path $LogPath -Value \"> service install (refused)`n$said\" -Encoding utf8; Run icacls.exe $Install /remove:g $SignedIn; Must ($code -ne 0) 'a folder others can change was not refused'; Must ((Status) -eq $NotInstalled) 'the refused install left a service behind'"
            }
            Self::BlockBackups => {
                "Run icacls.exe (Join-Path $Data $Backups) /deny \"${ServiceGroup}:(WD,AD)\""
            }
            Self::Install => "Run $Program service install --config $Settings",
            Self::NoRestart => {
                "WaitStatus $UpgradeStopped 180; $until = (Get-Date).AddSeconds(3 * $RestartSeconds); while ((Get-Date) -lt $until) { Must ((Status) -eq $UpgradeStopped) 'the service was started again'; Start-Sleep 2 }"
            }
            Self::Restart => {
                "Run icacls.exe (Join-Path $Data $Backups) /remove:d $ServiceGroup; Start-Service -Name $Name"
            }
            Self::Answers => "Answers 180",
            Self::Upgraded => {
                "Get-ChildItem -Name (Join-Path $Data $Backups) | Set-Content (Join-Path $Here $UpgradedBackups); Copy-Item (Join-Path $Data $VersionFile) (Join-Path $Here $UpgradedVersion)"
            }
            Self::SetupCode => {
                "Must ((Get-Item (Join-Path $Data $SetupCodeFile)).Length -gt 0) 'the setup code file is empty'"
            }
            Self::Status => "Must ((Status) -eq $Running) 'the service does not run'",
            Self::CrashRestart => {
                "$before = ServicePid; Stop-Process -Id $before -Force; Start-Sleep 2; WaitStatus $Running (3 * $RestartSeconds + 60); Answers 120; $after = ServicePid; Must (($after -ne 0) -and ($after -ne $before)) 'the service was not started again'"
            }
            Self::Uninstall => {
                "Run $Program service uninstall; Must ((Status) -eq $NotInstalled) 'the service is still there'; Must (Test-Path (Join-Path $Data $DatabaseFile)) 'the data did not stay'"
            }
        }
    }

    /// The commands of the step in bash, run with `set -e`; `skip` names why it does not apply.
    fn bash(self) -> &'static str {
        match self {
            Self::AppControl => "skip 'Linux has no Smart App Control'",
            Self::Verify => {
                "if [ -f \"$release/$checksums\" ]; then \"$next\" update verify \"$next\"; else skip 'the release folder is not signed'; fi"
            }
            Self::PreviousSetup => {
                "mkdir -p \"$install\"; chmod 755 \"$install\"; cp \"$previous_program\" \"$program\"; chmod 755 \"$program\"; \"$program\" setup --non-interactive --config \"$settings\" --data-dir \"$data\" --port \"$port\"; [ -f \"$settings\" ]"
            }
            Self::PreviousData => {
                "\"$program\" serve --config \"$settings\" >\"$here/previous-serve.txt\" 2>&1 & pid=$!; if ! answers 180; then kill \"$pid\"; false; fi; kill -INT \"$pid\"; wait \"$pid\" || true; [ -f \"$data/$database_file\" ]"
            }
            Self::Replace => "cp \"$next\" \"$program\"; chmod 755 \"$program\"",
            Self::RefuseOpen => {
                "chmod 777 \"$install\"; if \"$program\" service install --config \"$settings\"; then chmod 755 \"$install\"; echo 'a folder others can change was not refused' >&2; false; fi; chmod 755 \"$install\"; [ \"$(status)\" = \"$not_installed\" ]"
            }
            Self::BlockBackups => "chmod 500 \"$data/$backups\"",
            Self::Install => "\"$program\" service install --config \"$settings\"",
            Self::NoRestart => {
                "wait_status \"$upgrade_stopped\" 180; until=$((SECONDS + 3 * restart_seconds)); while [ \"$SECONDS\" -lt \"$until\" ]; do [ \"$(status)\" = \"$upgrade_stopped\" ] || { echo 'the service was started again' >&2; false; }; sleep 2; done; [ \"$(systemctl show \"$unit\" --property=NRestarts --value)\" = 0 ]"
            }
            Self::Restart => "chmod 700 \"$data/$backups\"; systemctl start \"$unit\"",
            Self::Answers => "answers 180",
            Self::Upgraded => {
                "ls -1 \"$data/$backups\" >\"$here/$upgraded_backups\"; cp \"$data/$version_file\" \"$here/$upgraded_version\""
            }
            Self::SetupCode => "[ -s \"$data/$setup_code_file\" ]",
            Self::Status => "[ \"$(status)\" = \"$running\" ]",
            Self::CrashRestart => {
                "before=\"$(main_pid)\"; systemctl kill --signal=KILL \"$unit\"; sleep 2; wait_status \"$running\" $((3 * restart_seconds + 60)); answers 120; after=\"$(main_pid)\"; [ \"$after\" != 0 ] && [ \"$after\" != \"$before\" ]"
            }
            Self::Uninstall => {
                "\"$program\" service uninstall; [ \"$(status)\" = \"$not_installed\" ]; [ -f \"$data/$database_file\" ]"
            }
        }
    }
}

/// What a script needs to know of its run.
pub(crate) struct Facts<'a> {
    /// The port the server listens on (`tryInstallServer` of `tools/dev/ports.json`).
    pub(crate) port: u16,
    /// The folder of the release and the program in it, as the guest sees them.
    pub(crate) release: &'a str,
    pub(crate) next_program: &'a str,
    /// The folder of the previous release's program and its file name, as the guest sees them.
    pub(crate) previous: &'a str,
    pub(crate) previous_program: &'a str,
    /// The name of the installed program.
    pub(crate) installed_name: &'a str,
}

/// The facts as `name = value` pairs, the same for both systems.
fn values(guest: Guest, facts: &Facts<'_>) -> Vec<(&'static str, String)> {
    let name = match guest {
        Guest::Windows => WINDOWS_NAME,
        Guest::Linux => SYSTEMD_UNIT,
    };
    let said = |sentence: message::Message| sentence.render();
    vec![
        ("port", facts.port.to_string()),
        ("release", facts.release.to_owned()),
        ("next_program", facts.next_program.to_owned()),
        ("previous", facts.previous.to_owned()),
        ("previous_program", facts.previous_program.to_owned()),
        ("installed_name", facts.installed_name.to_owned()),
        ("health", bb_api_words::health::LIVE.to_owned()),
        ("name", name.to_owned()),
        ("restart_seconds", RESTART_DELAY.as_secs().to_string()),
        ("checksums", CHECKSUMS_FILE.to_owned()),
        ("database_file", DATABASE_FILE.to_owned()),
        ("backups", BACKUPS_FOLDER.to_owned()),
        ("setup_code_file", SETUP_CODE_FILE.to_owned()),
        ("version_file", VERSION_FILE.to_owned()),
        ("upgraded_backups", UPGRADED_BACKUPS.to_owned()),
        ("upgraded_version", UPGRADED_VERSION.to_owned()),
        ("results_file", RESULTS_FILE.to_owned()),
        ("log_file", LOG_FILE.to_owned()),
        ("passed", PASSED.to_owned()),
        ("running", said(message::service_state_running(name))),
        (
            "upgrade_stopped",
            said(message::service_state_upgrade_stopped(name)),
        ),
        ("not_installed", said(message::service_not_installed(name))),
    ]
}

/// The PowerShell name of a fact: `release` is `$Release`, `next_program` is `$NextProgram`.
fn powershell_name(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut letters = part.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
        .collect()
}

/// A text in single quotes for PowerShell or bash.
fn quoted(guest: Guest, text: &str) -> String {
    match guest {
        Guest::Windows => format!("'{}'", text.replace('\'', "''")),
        Guest::Linux => format!("'{}'", text.replace('\'', r"'\''")),
    }
}

/// The whole script for `guest`.
pub(crate) fn script(guest: Guest, facts: &Facts<'_>) -> String {
    let mut text = String::new();
    for (name, value) in values(guest, facts) {
        let value = quoted(guest, &value);
        let _ = match guest {
            Guest::Windows => writeln!(text, "${} = {value}", powershell_name(name)),
            Guest::Linux => writeln!(text, "{name}={value}"),
        };
    }
    text.push_str(match guest {
        Guest::Windows => super::helpers::POWERSHELL,
        Guest::Linux => super::helpers::BASH,
    });
    for step in STEPS {
        let id = step.id();
        let _ = match guest {
            Guest::Windows => writeln!(text, "Step '{id}' {{ {} }}", step.powershell()),
            Guest::Linux => writeln!(
                text,
                "step_{} () {{ {}; }}\nstep '{id}' step_{}",
                id.replace('-', "_"),
                step.bash(),
                id.replace('-', "_")
            ),
        };
    }
    text.push_str(match guest {
        Guest::Windows => "Finish $true\n",
        Guest::Linux => "finish \"$passed\"\n",
    });
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts<'static> {
        Facts {
            port: 47_432,
            release: "rel'ease",
            next_program: "bugaboo-0.6.0-test-linux-x64",
            previous: "previous",
            previous_program: "bugaboo",
            installed_name: "bugaboo",
        }
    }

    #[test]
    fn ops_04_both_systems_run_the_same_steps_in_the_same_order() {
        for guest in [Guest::Windows, Guest::Linux] {
            let text = script(guest, &facts());
            let mut at = 0;
            for step in STEPS {
                let call = match guest {
                    Guest::Windows => format!("Step '{}'", step.id()),
                    Guest::Linux => format!("step '{}'", step.id()),
                };
                let found = text[at..]
                    .find(&call)
                    .unwrap_or_else(|| panic!("{guest:?}: {call} is missing or out of order"));
                at += found + call.len();
            }
        }
    }

    #[test]
    fn ops_04_the_scripts_take_the_words_and_names_of_the_program() {
        let windows = script(Guest::Windows, &facts());
        assert!(windows.contains("$Port = '47432'"), "{windows}");
        assert!(windows.contains("$Release = 'rel''ease'"));
        assert!(windows.contains(&format!(
            "$Running = '{}'",
            message::service_state_running(WINDOWS_NAME).render()
        )));
        let linux = script(Guest::Linux, &facts());
        assert!(linux.contains(r"release='rel'\''ease'"), "{linux}");
        assert!(
            linux.contains(&format!(
                "
name='{SYSTEMD_UNIT}'"
            )),
            "{linux}"
        );
        assert!(linux.contains(&format!("restart_seconds='{}'", RESTART_DELAY.as_secs())));
        assert!(linux.contains("health='/health/live'"));
        for text in [&windows, &linux] {
            for name in [DATABASE_FILE, BACKUPS_FOLDER, SETUP_CODE_FILE, VERSION_FILE] {
                assert!(text.contains(name), "{name}");
            }
        }
    }

    #[test]
    fn ops_04_every_step_has_its_own_name() {
        let mut ids: Vec<&str> = STEPS.iter().map(|step| step.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), STEPS.len());
    }
}
