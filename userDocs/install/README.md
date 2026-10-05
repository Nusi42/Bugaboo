# Install Bugaboo

Bugaboo is one program. You put it on a computer, answer a few questions, and start it. After that you do everything else in your browser.

Choose the page that fits your computer:

- [Install on Windows](windows.md)
- [Install on Linux](linux.md)
- [Install on Windows with WSL](wsl.md)

When Bugaboo runs, make your own account: [Your first sign-in](../sign-in/first-sign-in.md).

## What the questions are about

The command `bugaboo setup` asks these questions. Press Enter to accept the answer in brackets.

| Question | What to answer |
| --- | --- |
| Where should Bugaboo keep its data? | The path of a folder. Everything Bugaboo stores goes there. Back this folder up. |
| Who should be able to open Bugaboo? | `127.0.0.1` for this computer only, or `0.0.0.0` for everyone who can reach this computer. |
| Which port should Bugaboo listen on? | The suggestion is fine unless another program uses it. Bugaboo tells you when the port is busy. |
| How should the connection be secured? | `self-signed` lets Bugaboo make its own certificate. Browsers warn about it until you trust it. Choose `files` if you already have a certificate, or `off` only on this computer or behind a proxy that does HTTPS. |
| Which web address will people type? | The address people use in their browser, for example `https://bugaboo.example.com`. Passkeys belong to the host name of this address. |
| Which database? | `built-in` is right for a start. |

Your answers are saved in the settings file `bugaboo.toml`. You can run `bugaboo setup` again later. It asks before it replaces the file, or you add `--overwrite`.

To skip the questions, for example in a script, run `bugaboo setup --non-interactive` with the options you want. Run `bugaboo setup --help` to see them.

## The setup code

The first time Bugaboo starts, nobody has an account. So that only you can make the first one, Bugaboo makes a setup code. It works once and changes every time Bugaboo starts.

- When you start Bugaboo yourself with `bugaboo serve`, the code is shown in that window, on the line that starts with "Setup is open".
- When Bugaboo runs as a service, the code is in the file `setup-code.txt` inside the data folder.

## Keep the key files safe

Bugaboo keeps secrets in its database, such as the keys of code apps. The key files that open them are in the folder `keys` inside the data folder. A database backup without the key files cannot be read, and key files that lie around are a risk. Copy them to a safe place, away from the database backup:

```text
bugaboo keys export <folder>
```
