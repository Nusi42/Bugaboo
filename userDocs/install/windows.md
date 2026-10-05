# Install on Windows

1. Put `bugaboo.exe` in a folder of its own, for example `C:\Bugaboo`.
2. Open a terminal in that folder and run:

   ```text
   bugaboo setup
   ```

   Answer the questions. [What the questions are about](README.md#what-the-questions-are-about).
3. Start Bugaboo to try it:

   ```text
   bugaboo serve
   ```

   The window shows the address and the setup code. Leave it open and go to [Your first sign-in](../sign-in/first-sign-in.md). Press Ctrl+C to stop Bugaboo.

## Start with the computer

To have Bugaboo start with Windows, make it a service. Open the terminal as an administrator (right-click, "Run as administrator") and run:

```text
bugaboo service install
```

Bugaboo is now a Windows service named "Bugaboo". It runs as its own account, `NT SERVICE\Bugaboo`, which has no password. Windows starts it with the computer.

The service has no window, so the setup code is in the file `setup-code.txt` inside the data folder. Open it with Notepad and copy the code.

To remove the service, run this as an administrator. Your data stays where it is.

```text
bugaboo service uninstall
```

## The browser warns about the certificate

With `self-signed`, Bugaboo makes the certificate itself, so the browser does not know it. The warning is expected. The certificate is in the data folder. Add it to your browser or to Windows as trusted, or use your own certificate with the answer `files`.
