@echo off
rem Cargo's rustc-wrapper in the master repository only (ADR-0024): the master's ignored
rem .cargo/config.local.toml points here, so any build started in the master repository stops at once.
echo This is the master repository: nothing is built here. Work in a work folder (ADR-0024). 1>&2
exit /b 1
