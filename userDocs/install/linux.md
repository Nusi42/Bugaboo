# Install on Linux

1. Put the `bugaboo` program in a folder, for example `/opt/bugaboo`, and make it runnable with `chmod +x /opt/bugaboo/bugaboo`.
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

This makes the user `bugaboo` (no password, no shell, no home folder) if it does not exist, gives it the data folder from your settings, installs the unit `bugaboo.service` and starts it. Your settings file stays where it is, the one `bugaboo setup` wrote or the one you name with `--config`. The service reads it, so the program, the settings file and the folders above them must be readable for the user `bugaboo`. Bugaboo makes the file readable for that user and nobody else, because it can hold a database password. The unit lets Bugaboo write only into its data folder.

The service has no window, so the setup code is in the file `setup-code.txt` inside the data folder. Read it with `sudo cat`.

Useful commands:

```text
sudo systemctl status bugaboo
sudo journalctl -u bugaboo
sudo /opt/bugaboo/bugaboo service uninstall
```

`service uninstall` removes the service. Your data stays where it is.

## The browser warns about the certificate

With `self-signed`, Bugaboo makes the certificate itself, so the browser does not know it. The warning is expected. The certificate is in the data folder. Trust it in your browser, or use your own certificate with the answer `files`.
