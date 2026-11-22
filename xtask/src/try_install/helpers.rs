//! The helpers of the two scripts of the try-out, after the facts of the run and before its steps
//! (`steps.rs`). They find the folders the system names (the program folder of Windows, the
//! folder systemd names for the state of services), run the program and log what it says, wait
//! until the server answers or the service is in a state, and write each step's result.

/// PowerShell, for Windows Sandbox. The script lies in the results folder; the release folder and
/// the folder of the previous program are mapped beside it.
pub(super) const POWERSHELL: &str = r#"param([switch]$CloseWhenDone)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$Here = $PSScriptRoot
$Release = Join-Path (Split-Path $Here) $Release
$Previous = Join-Path (Split-Path $Here) $Previous
$Install = Join-Path $env:ProgramFiles $Name
$Program = Join-Path $Install $InstalledName
$Settings = Join-Path $Install 'bugaboo.toml'
$Data = Join-Path $Install 'data'
$Next = Join-Path $Release $NextProgram
$PreviousProgram = Join-Path $Previous $PreviousProgram
$Url = "https://127.0.0.1:$Port$Health"
$ResultsPath = Join-Path $Here $ResultsFile
$LogPath = Join-Path $Here $LogFile
Set-Content -Path $ResultsPath -Value @() -Encoding utf8
Set-Content -Path $LogPath -Value @() -Encoding utf8

function Note([string]$Line) { Add-Content -Path $ResultsPath -Value $Line -Encoding utf8 }
function Finish([bool]$Ok) {
    if ($Ok) { Note $Passed } else { Note 'failed' }
    if ($CloseWhenDone) { shutdown.exe /s /t 0 }
    exit ([int](-not $Ok))
}
function Must([bool]$Holds, [string]$Why) { if (-not $Holds) { throw $Why } }
function Run {
    $exe = $args[0]
    $rest = @($args | Select-Object -Skip 1)
    $old = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $out = (& $exe @rest 2>&1 | ForEach-Object { "$_" }) -join "`n"
    $code = $LASTEXITCODE
    $ErrorActionPreference = $old
    Add-Content -Path $LogPath -Value "> $exe $($rest -join ' ')`n$out" -Encoding utf8
    if ($code -ne 0) { throw "$exe $($rest -join ' ') ended with $code`: $out" }
    $out.Trim()
}
function Status { Run $Program service status }
function WaitStatus([string]$Want, [int]$Seconds) {
    $until = (Get-Date).AddSeconds($Seconds)
    do { $now = Status; if ($now -eq $Want) { return }; Start-Sleep 2 } while ((Get-Date) -lt $until)
    throw "the service stayed: $now"
}
function Answers([int]$Seconds) {
    $until = (Get-Date).AddSeconds($Seconds)
    do {
        $code = & curl.exe -k -s -o NUL -w '%{http_code}' $Url
        if ($code -eq '200') { return }
        Start-Sleep 2
    } while ((Get-Date) -lt $until)
    throw "no answer on $Url"
}
# The group every service account belongs to (NT AUTHORITY\SERVICE). Denying it to add files
# and folders in one folder, without handing the entry on, stops the service there and leaves
# the install's own `icacls /T` working; an entry for the service's own SID would be replaced by
# the install's grant to it.
$ServiceGroup = '*S-1-5-6'
# The signed-in accounts (NT AUTHORITY\Authenticated Users), which a folder made directly on the
# system drive lets change what is in it: the install must refuse a program folder they may change.
$SignedIn = '*S-1-5-11'
function ServicePid { [int](Get-CimInstance Win32_Service -Filter "Name='$Name'").ProcessId }
function Step([string]$Id, [scriptblock]$Body) {
    $script:Skipped = $null
    try { & $Body | Out-Null } catch { Note "failed ${Id}: $($_.Exception.Message)"; Finish $false }
    if ($script:Skipped) { Note "skipped ${Id}: $script:Skipped" } else { Note "ok $Id" }
}
"#;

/// bash, for WSL, run as root with the release folder and the folder of the previous program as
/// the facts name them. The program goes into the folder systemd names for the state of system
/// services, outside every home folder, which the unit hides.
pub(super) const BASH: &str = r#"set -u
here="$(cd "$(dirname "$0")" && pwd)"
unit="$name"
install="$(systemd-path system-state-private)/bugaboo"
program="$install/$installed_name"
settings="$install/bugaboo.toml"
data="$install/data"
next="$release/$next_program"
previous_program="$previous/$previous_program"
url="https://127.0.0.1:$port$health"
results="$here/$results_file"
log="$here/$log_file"
: >"$results"
: >"$log"

note() { printf '%s\n' "$1" >>"$results"; }
finish() { note "$1"; if [ "$1" = "$passed" ]; then exit 0; else exit 1; fi; }
skip() { printf '%s' "$1" >"$here/skipped"; }
status() { "$program" service status; }
main_pid() { systemctl show "$unit" --property=MainPID --value; }
wait_status() {
    until=$((SECONDS + $2))
    while :; do
        now="$(status 2>&1 || true)"
        [ "$now" = "$1" ] && return 0
        [ "$SECONDS" -ge "$until" ] && { echo "the service stayed: $now" >&2; return 1; }
        sleep 2
    done
}
answers() {
    until=$((SECONDS + $1))
    while :; do
        code="$(curl -k -s -o "$here/answer.txt" -w '%{http_code}' "$url" || true)"
        [ "$code" = 200 ] && return 0
        [ "$SECONDS" -ge "$until" ] && { echo "no answer on $url" >&2; return 1; }
        sleep 2
    done
}
step() {
    rm -f "$here/skipped"
    ( set -e; "$2" ) >"$here/step.txt" 2>&1
    code=$?
    { printf '== %s\n' "$1"; cat "$here/step.txt"; } >>"$log"
    if [ "$code" -ne 0 ]; then
        note "failed $1: $(tail -n 3 "$here/step.txt" | tr '\n' ' ')"
        finish failed
    elif [ -f "$here/skipped" ]; then
        note "skipped $1: $(cat "$here/skipped")"
    else
        note "ok $1"
    fi
}
"#;
