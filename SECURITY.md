# Security

## Reporting a problem

If you find a security problem in Bugaboo, please report it privately, not
in a public issue:

1. Open the [Security tab](https://github.com/Nusi42/Bugaboo/security) of
   this repository.
2. Choose **Report a vulnerability** and describe what you found.

Only the maintainer sees the report. Please include what is needed to
understand and repeat the problem: the version or commit, the steps, what
you expected and what happened, and how serious you think it is.

You will get an answer within a week. We fix the problem and publish it in
a coordinated way: within 90 days of your report, or sooner when the fix is
out, with a security advisory on GitHub that names you as the finder if you
wish. Please keep the problem to yourself until then.

Every Bugaboo server also answers `/.well-known/security.txt` (RFC 9116).
It names the organization that runs the server first, when it has set a
contact, and then this page's address: report a problem with one
installation to its organization, and a problem in Bugaboo itself here.

## Which versions get fixes

<!-- BEGIN GENERATED support-period by `cargo xtask gen release` from bb-core and Cargo.toml; edit those, not this text -->

Version 0.6.0, released on 2026-11-24, is a free public preview. Until 1.0, only the newest release gets security fixes; a fix comes as the next patch release. From 1.0 on, each minor release gets security fixes for 5 years from the day its first release (X.Y.0) comes out, as patch releases without new features.

<!-- END GENERATED support-period -->

A security fix ships as a patch release made from the release's tag on a
branch `release-X.Y`, without new features. Bugaboo does not tell you about
new releases by itself yet: watch the releases of this repository.

## Checking a download

<!-- BEGIN GENERATED release-keys by `cargo xtask gen release` from bb-core and Cargo.toml; edit those, not this text -->

The files of a release are signed with one of these public keys (minisign, Ed25519). A file signed with either one is genuine.

| Key | Public key |
|-----|------------|
| release | `RWRj80xzx+o/0lgZdGYG/LB1xPfDN/Nxw+GFJiI0k7ofdUATvirpUB8Y` |
| backup | `RWTSVLiNDLni7T+d8XyvzayQo73d6ldQUfdAt/hZ9jKDACJfdy5JPEEP` |

Check a downloaded file with the Bugaboo you have installed (`bugaboo update verify <file>`), or with the minisign tool (`minisign -Vm <file> -P <public key>`).

<!-- END GENERATED release-keys -->

How to check a download step by step is in
[Check a download](userDocs/install/verify-a-download.md).

## What we report to the authorities

Bugaboo is made in the European Union. When a vulnerability in Bugaboo is
actively exploited, or a severe incident affects its security, we report
it through ENISA's single reporting platform as the Cyber Resilience Act
asks: an early warning within 24 hours of learning of it, a notification
within 72 hours, and a final report once a fix or a mitigation is out
(14 days after it for a vulnerability, one month after the notification for
an incident). How we do that step by step is in the
[vulnerability runbook](docs/security/vulnerability-runbook.md).

## What is covered

Please do not test against servers that other people run, and do not read,
change or delete data that is not yours while you look for a problem.

## How Bugaboo is built for security

The decisions about security are in
[`docs/architecture/12-security.md`](docs/architecture/12-security.md), and
the checks that keep them are listed in
[`docs/architecture/10-checks.md`](docs/architecture/10-checks.md). What
meets each essential requirement of the Cyber Resilience Act is in
[`docs/security/cra-requirements.md`](docs/security/cra-requirements.md).
