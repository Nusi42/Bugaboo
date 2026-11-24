# Check a download

Every Bugaboo release has a list of its files with a checksum for each,
`SHA256SUMS`, and one signature for that list, `SHA256SUMS.minisig`. Check a
file before you install it: the signature proves that the list comes from the
makers of Bugaboo, and the checksum in the list proves that nobody changed
the file on the way.

## What you need

Download these files of the release into one folder:

- the program, for example `bugaboo-0.6.0-windows-x64.exe` or
  `bugaboo-0.6.0-linux-x64`;
- `SHA256SUMS` and `SHA256SUMS.minisig`.

## With the Bugaboo you have installed

When Bugaboo already runs on the computer, let it check the new program:

```text
bugaboo update verify bugaboo-0.6.0-linux-x64
```

The installed program trusts only the keys it carries itself, so a download
cannot bring a key of its own. It answers one of these:

- **Signed by Bugaboo, version 0.6.0:** the file is genuine. Install it.
- **… version … is older than version …, which is installed here:** the file
  is genuine but older than what you run, and the command ends with an error,
  so a script does not take it for an update. Installing it would go back to
  an older version, which can undo fixes of security. If you mean to go back,
  run the command again with `--allow-older`: it then says the file is signed
  and warns. A test version such as `0.6.0-rc1` counts as older than `0.6.0`.
- **SHA256SUMS was changed after it was signed**, **SHA256SUMS is not
  signed with a key of Bugaboo**, **… is not in the list SHA256SUMS** or
  **The checksum of … is not the one in the list:** do not install the file.
  Download it and the list again from the release page, and report it if the
  check fails again.

The check only reads the files. It never runs, moves or replaces anything.

## With the minisign tool

On a computer without Bugaboo, check the list with the common
[minisign](https://jedisct1.github.io/minisign/) tool and a public key from
[Checking a download in SECURITY.md](../../SECURITY.md#checking-a-download):

```text
minisign -Vm SHA256SUMS -P <public key>
```

It says `Signature and comment signature verified` and shows the name
`SHA256SUMS` and the version that were signed. If it says anything else, do
not install anything from the release.

Then check the file against the list. On Linux:

```text
sha256sum -c SHA256SUMS --ignore-missing
```

It names each file of the list that is in the folder with `OK`. On Windows,
in PowerShell:

```powershell
$sum = (Get-FileHash bugaboo-0.6.0-windows-x64.exe -Algorithm SHA256).Hash.ToLower()
Select-String -Path SHA256SUMS -SimpleMatch "$sum  bugaboo-0.6.0-windows-x64.exe"
```

It shows the line of the file in the list. If a file is not `OK`, or
PowerShell shows no line, do not install the file.

## Windows warns about an unknown publisher

Windows does not know the program yet, because it is not signed with a
certificate that Windows trusts. Check the signature as above first; when it
holds, choose **More info** and then **Run anyway**.
