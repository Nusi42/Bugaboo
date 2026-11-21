# Making a release

How a release of Bugaboo is built, signed, checked and published
(ADR-0047). A release is a tag `vX.Y.Z` on `main` with the version of
[ADR-0040](../adr/0040-version-numbers.md). The steps marked *maintainer*
need the secret key and are never done by an agent.

## The keys (once)

Releases are signed with one of two Ed25519 keys in the minisign format:
the **release key**, used for every release, and the **backup key**, kept
apart and used only when the release key is lost or leaked. Both public
keys are written once, in `RELEASE_PUBLIC_KEYS` of
`crates/bb-core/src/release/signature.rs`; `SECURITY.md` and the release
notes get them from there.

1. *Maintainer:* make each key on a computer you trust, into a place outside
   the repository and every work folder:

   ```text
   cargo xtask release keygen --out <path>/bugaboo-release.key
   cargo xtask release keygen --out <path>/bugaboo-backup.key
   ```

   Each asks nothing and prints the public key. The keys have no passphrase.
   The command refuses a place inside a repository or a work folder, never
   writes over a key, and makes the secret key file readable by its owner
   only; `release sign` refuses a key file that other accounts may read and
   names the command that closes it. Keep the release key and the backup key
   apart from each other and safe; if one may be known to somebody else,
   replace it as *Replace a key* below says.
2. Put the two public keys into `RELEASE_PUBLIC_KEYS` (`release` first,
   `backup` second), set `KEYS_ARE_FOR_TESTS` to `false`, and run
   `cargo xtask gen release`, which writes them into `SECURITY.md`.

Until then the list holds test keys whose secret keys nobody has, and every
release folder, the name of every file in it, the version in every signature
and the notes say *test*.

## Each release

1. In the commit that records the finished milestone: the version in
   `[workspace.package]` and the day of the release in
   `[workspace.metadata.release]` of `Cargo.toml` (`date`, the day of this
   commit, which the tag will name; for a new minor version `X.Y.0` also
   `minor-date`, the same day, from which the support period counts), then
   `cargo xtask gen release`.
2. Write the notes as `docs/releases/<version>.md`, starting from
   [TEMPLATE.md](TEMPLATE.md). Keep its `BEGIN GENERATED` comments: the
   build writes the keys, the support period and the kind of build there.
3. Read the [vulnerability runbook](../security/vulnerability-runbook.md) and
   the [CRA mapping](../security/cra-requirements.md) again, and check that
   nothing in them is out of date.
4. Tag the commit `v<version>` on `main`.
5. Build the folder on the Windows computer, with WSL and its Rust (with the
   target `x86_64-unknown-linux-musl`) installed:

   ```text
   cargo xtask release
   ```

   It builds the Windows program with `cargo xtask build`, the static Linux
   program in WSL (`cargo xtask release --linux` there, in a build folder of
   its own, `target/wsl`), and writes the parts list, the notices, the VEX
   statements and the notes into `target/release-folder/<version>/`. Without
   the tag on the commit, or while the program trusts only test keys, it is a
   test build: the folder, every file and the version in every signature are
   named `<version>-test`. When the tag exists, it refuses a day of the
   release that is not the day of the tag's commit.

   The Linux program is built in the computer's default WSL distribution.
   When that one belongs to something else, name the distribution for
   Bugaboo's builds in the ignored file `.env.local` of the checkout:

   ```text
   BUGABOO_WSL_DISTRIBUTION=<name>
   ```

   Every WSL call of the release then goes to it (`wsl.exe -d <name>`). It
   needs Rust with the target `x86_64-unknown-linux-musl` and `musl-gcc`.
   The name belongs to that computer and is never committed.
6. *Maintainer:* sign it with the release key:

   ```text
   cargo xtask release sign --key <your key folder>/bugaboo-release.key
   ```

   It writes `SHA256SUMS` and a `.minisig` for every file and checks each.
   The key may also be named in `BUGABOO_RELEASE_KEY`. Once the program
   carries the real keys, it refuses a folder built without the tag and a
   key that is not one of `RELEASE_PUBLIC_KEYS`.
7. Check the folder against the keys the program carries:

   ```text
   cargo xtask release check
   ```

8. *Maintainer:* publish the folder as a GitHub release of the tag: every
   file of the folder, with the notes as the text of the release.

## A security fix (patch release)

Until 1.0, only the newest release gets fixes, as the next patch release.
From 1.0 on, a fix for an older minor release is made on a branch
`release-X.Y` cut at its tag (`git switch -c release-X.Y vX.Y.0`), with the
fix and nothing else, and released from there with the steps above.

## Replace a key

When the release key (or the backup key) is lost, or may be known to
somebody else:

1. Sign the next release with the **other** key. Installed programs trust
   both, so they accept it.
2. In the same release, remove the bad key from `RELEASE_PUBLIC_KEYS`, make
   a new backup key with `cargo xtask release keygen`, add its public key,
   and run `cargo xtask gen release`.
3. Say in the notes which key was dropped and why. When a key may be known
   to somebody else, publish a security advisory too: files signed with it
   after the date of the leak are not genuine.
4. The key that signed this release is the release key from now on.
