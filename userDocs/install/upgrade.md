# Upgrade Bugaboo

To upgrade, stop Bugaboo, replace the program with the new version, and start it again. The new version changes the database to its own form by itself. Before it changes anything, it creates a backup, so there is always a way back.

## Step by step

1. Download the new program, its signature, `SHA256SUMS` and `SHA256SUMS.minisig` from the release page into one folder.
2. Check it with the Bugaboo you have installed:

   ```text
   bugaboo update verify bugaboo-0.6.0-linux-x64
   ```

   Go on only when it says **Signed by Bugaboo**. When it warns that the version is older than yours, you are about to go back to an older version: stop, unless you mean to. More in [Check a download](verify-a-download.md).
3. Stop Bugaboo: stop the service (`Bugaboo` on Windows, `bugaboo.service` on Linux), or press Ctrl+C where it runs in a window.
4. Put the new program in place of the old one, with the same name.
5. Start Bugaboo again.

## What happens at the start

When the new version starts on the data of an older one:

1. It sees that the database lacks some of its changes. It compares every change the database has with its own list, not only the newest one, so a change that was left out in between is found too.
2. It creates a backup of the database and the attached files in the `backups` folder of the data folder, and says where: `bugaboo-before-upgrade-<moment>-<version>.backup`, with its own version in the name. Like every backup, it holds no keys, and only the account that runs Bugaboo can open it.
3. It changes the database, one change after the other, and starts.

Bugaboo records the backup like every other, with the upgrade as what created it, and the security log has a line for it. Bugaboo keeps the newest 3 backups created before an upgrade (*Backups before an upgrade to keep* in *Organization settings › System › Backups*) and deletes older ones with the backups of every day. They always go into the `backups` folder of the data folder, also when the backups of every day go into another folder.

A new installation creates no backup: there is nothing to keep yet. A start of the same version creates none either.

An upgrade keeps AI apps and agents as they were; new installations start with both off. An administrator switches them on in *Organization settings › Connections*.

## When something goes wrong

Bugaboo stops before it serves anybody and says in plain words what happened. It ends with an exit code of its own, so a service manager can tell that starting it again does not help.

- **The backup failed.** Nothing was changed. The message says why, for example that the drive is full. Free up room or fix what the message names, and start Bugaboo again.
- **A change of the database failed.** Bugaboo puts back the backup it created first and stops. The database is as it was before the start. The message names what failed, where the backup is, and the version you used before: that version works with this database. Start it again and report the problem.
- **Putting the backup back failed too.** The message says so. The database may be half changed: do not start Bugaboo on it. Keep the backup safe and report the problem; the backup can be restored as described in [Backups and restore](backups.md).
- **The database belongs to a newer version.** A version of Bugaboo cannot work on a database that a newer version changed. The message names the newer version when it knows it: start that version, or a newer one.
- **The database has a change this version does not know.** Another version of Bugaboo changed the database in a way this one does not have. Nothing was changed: start the version that last worked on the database.
- **An earlier upgrade did not finish.** When the computer stopped while Bugaboo changed the database, the database may be half changed, and the version you used before may not work with it. If the next start stops too, its message says so: do not start Bugaboo on this database again. Keep the backup the message names, which Bugaboo created before that first upgrade: it holds the database as the version you used before left it. Report the problem.

Bugaboo writes the version that last started on the data folder into the file `version.txt` in it. That is how a message can name the version you used before. While it changes the database, the file also names the backup it created first, so a later start can tell that an upgrade did not finish.

## Before you upgrade

- Read the release notes of the new version: they say what changed and what to look out for. Until 1.0, only the newest release gets security fixes, so keep up with the releases.
- Create a backup of your own and keep it with the exported keys (`bugaboo keys export <folder>`), as for any change: [Backups and restore](backups.md).
- Check that the drive of the data folder has room for one more backup. [How much room a backup takes](backups.md#how-much-room-a-backup-takes) says how to tell.

Every version of Bugaboo is tested before its release by upgrading a database that the previous version created, on both kinds of database, and by checking that everything reads the same afterwards.
