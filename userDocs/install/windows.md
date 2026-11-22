# Install on Windows

Windows 11 with *Smart App Control* switched on (new computers start with it on) does not start `bugaboo.exe` yet: the program is not signed for Windows, and Windows says that a policy blocked it. Use a computer where Smart App Control is off, or wait for a release that is signed for Windows.

1. Put `bugaboo.exe` in a folder of its own, for example `C:\Program Files\Bugaboo`. The program needs nothing else installed. If you want to run it as a service later, do not use a folder inside your own user folder (such as Downloads or Desktop): the service runs as an account of its own, which cannot read there.
2. Open a terminal as an administrator (a folder in Program Files needs it) in that folder and run:

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

To have Bugaboo start with Windows, run it as a service. Open the terminal as an administrator (right-click, "Run as administrator") and run:

```text
bugaboo service install
```

Bugaboo is now a Windows service named "Bugaboo". It runs as its own account, `NT SERVICE\Bugaboo`, which has no password. Windows starts it with the computer.

The install first checks that only the administrators of the computer can change the program, its folder, the folders above it and the folder of the settings file, and that this account can run the program. If not, for example because they are in your user folder or in a folder created directly on drive `C:`, it says which path and installs nothing: move both to a folder such as `C:\Program Files\Bugaboo` and install from there. Then it lets the account of the service read the settings file and the administrators change it, and nobody else. [Rights of the program and its settings](README.md#rights-of-the-program-and-its-settings) says why.

If Bugaboo fails while it runs, Windows starts it again after 10 seconds, but at most 3 times a day: after the third failure in a day it stays stopped until you start it, and `bugaboo service status` says that it ended unexpectedly. Windows does not start it again at all when Bugaboo stopped at an upgrade: starting again would stop the same way. Then Bugaboo writes what happened and what to do into the file `logs\server.log` in the data folder; fix that and start the service again in the Services window. [Upgrade Bugaboo](upgrade.md) explains the messages.

To see how the service is, run:

```text
bugaboo service status
```

It says whether the service is installed, whether it runs, and whether it stopped at an upgrade.

The service has no window, so the setup code is in the file `setup-code.txt` inside the data folder. Open it with Notepad and copy the code.

To remove the service, run this as an administrator. Your data stays where it is.

```text
bugaboo service uninstall
```

## The browser warns about the certificate

With `self-signed`, Bugaboo creates the certificate itself, so the browser does not know it. The warning is expected. The certificate is in the data folder. Add it to your browser or to Windows as trusted, or use your own certificate with the answer `files`.
