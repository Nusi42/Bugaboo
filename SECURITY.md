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

You will get an answer within a week. Once the problem is fixed, the fix and
a short note are published, and you are named as the finder if you wish.

## What is covered

Bugaboo is not released yet (see [Where it stands](README.md#where-it-stands)).
Until version 1.0, only the newest state of the `main` branch gets fixes.

Please do not test against servers that other people run, and do not read,
change or delete data that is not yours while you look for a problem.

## How Bugaboo is built for security

The decisions about security are in
[`docs/architecture/12-security.md`](docs/architecture/12-security.md), and
the checks that keep them are listed in
[`docs/architecture/10-checks.md`](docs/architecture/10-checks.md).
