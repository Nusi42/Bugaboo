# Check a download

Every file of a Bugaboo release is signed. Check a file before you install
it: the check proves that the file comes from the makers of Bugaboo and that
nobody changed it on the way.

## What you need

Download these files of the release into one folder:

- the program, for example `bugaboo-0.6.0-windows-x64.exe` or
  `bugaboo-0.6.0-linux-x64`;
- its signature, the same name with `.minisig` at the end;
- `SHA256SUMS` and `SHA256SUMS.minisig`.

## With the Bugaboo you have installed

When Bugaboo already runs on the computer, let it check the new program:

```text
bugaboo update verify bugaboo-0.6.0-linux-x64
```

The installed program trusts only the keys it carries itself, so a download
cannot bring a key of its own. It answers one of these:

- **Signed by Bugaboo, version 0.6.0:** the file is genuine. Install it.
- **Version … is older than version …, which is installed here:** the file
  is genuine but older than what you run. Installing it would go back to an
  older version; do that only if you mean to.
- **… was changed after it was signed**, **… is not signed with a key of
  Bugaboo**, **… is not in the list SHA256SUMS** or **The checksum of … is
  not the one in the list:** do not install the file. Download it again from
  the release page, and report it if the check fails again.

The check only reads the files. It never runs, moves or replaces anything.

## With the minisign tool

On a computer without Bugaboo, use the common
[minisign](https://jedisct1.github.io/minisign/) tool with a public key from
[Checking a download in SECURITY.md](../../SECURITY.md#checking-a-download):

```text
minisign -Vm bugaboo-0.6.0-linux-x64 -P <public key>
```

It says `Signature and comment signature verified` and shows the file name
and the version that were signed. If it says anything else, do not install
the file.

## Windows warns about an unknown publisher

Windows does not know the program yet, because it is not signed with a
certificate that Windows trusts. Check the signature as above first; when it
holds, choose **More info** and then **Run anyway**.
