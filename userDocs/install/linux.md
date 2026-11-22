# Install on Linux

1. Put the `bugaboo` program in a folder, for example `/opt/bugaboo`, and mark it as runnable with `chmod +x /opt/bugaboo/bugaboo`.
2. Run the questions:

   ```text
   /opt/bugaboo/bugaboo setup
   ```

   Answer them. [What the questions are about](README.md#what-the-questions-are-about).
3. Start Bugaboo to try it:

   ```text
   /opt/bugaboo/bugaboo serve
   ```

   The window shows the address and the setup code. Leave it open and go to [Your first sign-in](../sign-in/first-sign-in.md). Press Ctrl+C to stop Bugaboo.

## Start with the computer

On a Linux computer with systemd, run this with administrator rights:

```text
sudo /opt/bugaboo/bugaboo service install
```

This creates the user `bugaboo` (no password, no shell, no home folder) if it does not exist, gives it the data folder from your settings, installs the unit `bugaboo.service` and starts it. Your settings file stays where it is, the one `bugaboo setup` wrote or the one you name with `--config`. The service reads it, so the program, the settings file and the folders above them must be readable for the user `bugaboo`. Bugaboo lets only that user read the file, because it can hold a database password. For that the settings file must not be a link, must not be inside the data folder, and must be in a folder that belongs to root and that nobody else can write in, for example `/etc/bugaboo`. The program, its folder and every folder above it must belong to root and nobody else may write to them, and the program must not be inside the data folder: otherwise the install refuses and says which path. [Rights of the program and its settings](README.md#rights-of-the-program-and-its-settings) says why. If the data folder is a link, the service uses the folder it leads to.

The unit keeps Bugaboo on a short leash. It may write only into its data folder, cannot see the home folders (`/home`, `/root`) or the hardware of the computer, holds no special rights and cannot gain any, and opens only connections to other computers and to programs on this one. So:

- Keep the program, the settings file and the data folder outside the home folders, for example in `/opt/bugaboo`, `/etc/bugaboo` and `/var/lib/bugaboo`. The install asks systemd whether the service would see them, and refuses and says which one it would not.
- Use a port of 1024 or above, such as the suggested 8443. A port below 1024 needs a right the service does not have; put a reverse proxy in front if people should reach Bugaboo on port 443.

If Bugaboo fails while it runs, systemd starts it again after 10 seconds, as often as it fails (Windows stops after 3 times a day). It does not do so when Bugaboo stopped at an upgrade: starting again would stop the same way. `sudo journalctl -u bugaboo` shows what happened and what to do; fix that and run `sudo systemctl start bugaboo`. [Upgrade Bugaboo](upgrade.md) explains the messages.

The service has no window, so the setup code is in the file `setup-code.txt` inside the data folder. Read it with `sudo cat`.

Useful commands:

```text
/opt/bugaboo/bugaboo service status
sudo systemctl status bugaboo
sudo journalctl -u bugaboo
sudo /opt/bugaboo/bugaboo service uninstall
```

`service uninstall` removes the service. Your data stays where it is.

## The browser warns about the certificate

With `self-signed`, Bugaboo creates the certificate itself, so the browser does not know it. The warning is expected. The certificate is in the data folder. Trust it in your browser, or use your own certificate with the answer `files`.
