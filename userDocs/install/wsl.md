# Install on Windows with WSL

WSL runs Linux inside Windows. Bugaboo works there like on [Linux](linux.md). Two things differ.

## Switch systemd on

A service needs systemd. In WSL 2, add these lines to the file `/etc/wsl.conf`, then run `wsl --shutdown` in a Windows terminal and open WSL again:

```text
[boot]
systemd=true
```

Without systemd you can still start Bugaboo yourself with `bugaboo serve`. It then stops when you close the WSL window.

## Open it from Windows

Windows reaches a server in WSL at `localhost`. Choose the address `127.0.0.1` in the questions, and open `https://localhost:<port>` in your Windows browser, with the port you chose.

Passkeys belong to a host name. Use the same name each time you sign in, for example always `localhost`.

Then follow the steps for [Linux](linux.md), and go to [Your first sign-in](../sign-in/first-sign-in.md).
