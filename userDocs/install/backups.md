# Backups and restore

A backup is one file that holds the organization's data and its attached files at one moment. It holds no keys. Bugaboo creates backups while people go on working, so you never have to stop it for one.

## What a backup holds

- The database: every issue, comment, setting and person, as they were at the moment of the backup.
- The attached files that the database points to. A file that somebody adds after that moment is not in the backup.
- A copy of the replay list: the people who were removed or switched off, and the keys, accesses of AI apps and passkeys that were taken back. A restore does these again, so an old backup cannot bring them back.

A backup holds no keys. Back up the keys apart from the backups with `bugaboo keys export <folder>`, and keep them in another safe place. Whoever has a backup and the keys can read everything Bugaboo stores.

The file is called `bugaboo-<moment>-<version>.backup`, for example `bugaboo-20261118T031500Z-0.6.0.backup`. The moment is in UTC. A backup that a new version created before it changed the database is called `bugaboo-before-upgrade-<moment>-<version>.backup`: [Upgrade Bugaboo](upgrade.md).

## Where backups go

Backups go into the `backups` folder of the data folder. An administrator can name another folder in *Organization settings › System › Backups* (*Folder of the backups*), and `bugaboo backup --to <folder>` puts one backup somewhere else. The folder must be one that only the account that runs Bugaboo can open: Bugaboo refuses a folder that other accounts can open. The folder of the settings must also be on a disk of the server itself: Bugaboo refuses a shared folder of another computer (`\\server\share`, `//server/share`, or a drive letter or a mount that leads to one, where the system tells), when it is saved and when a backup runs, because the server would sign in to the other computer with its own account and copy the whole database there. To keep backups on another computer, copy them there, or run `bugaboo backup --to <folder>` on the server, which is your choice as the person who runs the server. A change of the folder, like every setting of backups, is in the security log.

Backups are as private as the data folder. Copy them to another disk or computer as well, so a broken disk does not take the data and the backups together.

## Backups every day

Bugaboo creates a backup every night at the quiet hour of the organization (*Create a backup every day*, on at the start). It keeps the newest backup of each of the last 7 days and of each of the last 4 weeks (*Daily backups to keep*, *Weekly backups to keep*) and deletes the others; the days and weeks are those of the time zone of the organization (*Region › Time zone*), or UTC when none is set. A backup that a restore created of what was there before is kept for the keep period of the security log (*Organization settings › Security*), whatever these settings say. Of the backups created before an upgrade, the newest 3 are kept (*Backups before an upgrade to keep*). A file in the folder whose name Bugaboo did not create is never deleted. A stored file that is missing from the disk while a backup runs is left out of it, and the server log says which.

## Create a backup now

On the command line of the server:

```text
bugaboo backup
```

It says the name, the size and the folder of the new backup. `bugaboo backup list` lists the backups in the folder.

People with *Change organization settings* can create one from the page *Organization settings › System › Backups* too (see "The Backups page" below). Every backup is written to the security log with its name, who created it and its checksum.

## The Backups page

Open *Organization settings*, then "Backups". It needs *Change organization settings*.

- At the top it says where backups are and that each one holds all the data of the organization and its attached files, but no keys.
- "Daily backups" has the same settings as above: whether Bugaboo creates a backup every day, and how many daily and weekly backups it keeps. A change is saved at once.
- **Create a backup now** creates a backup while people go on working. The button says it is busy until the backup is there. The new backup then comes in at the top of the list, and nothing else on the page moves. If another backup is being created, the page says so: wait until it is done and try again.
- "All backups" lists the backups newest first: the name, the moment, the size and who or what created it (the daily backup, a person, the command line, an upgrade, or a restore). Click a backup to choose it.
- "Restore a backup" shows the command that restores the chosen backup, with its name filled in. Copy it into the command line of the server. The page has no button that restores, because a restore needs the server to be stopped.

## Restore a backup

A restore puts the data of a backup back in place of what is there now. It runs on the command line of the server, with the server stopped, as the account that runs Bugaboo (the same as `bugaboo rotate-key`).

1. Stop Bugaboo (the service, or the window where `bugaboo serve` runs).
2. Run:

   ```text
   bugaboo restore <the backup file>
   ```

3. Start Bugaboo again.

The restore:

- checks the whole backup first: a damaged backup, one from a newer Bugaboo, or one of another kind of database is refused, and nothing changes;
- checks that this installation's keys sealed the backup, so a file that another installation or somebody else wrote, or changed, is refused, and nothing changes;
- checks that the keys of this installation are the keys of the backup and open its stored secrets; when they are not, it stops before anything changes and says which command to run;
- creates a backup of what is there now, so you can go back to it the same way;
- puts the data and the files back, and brings an older backup up to this version of Bugaboo;
- signs everybody out;
- does again what the replay list says (see below);
- checks the security log, writes into it what was restored and which entries are gone, and starts its anchor again from the restored log, so the status page stays green.

Every other change after the moment of the backup is gone, password and role changes included. The restore says so at the end: tell the people concerned.

If the restore stops half way, it says so, and names the backup of what was there before with the command that puts it back.

### On another computer

1. Install Bugaboo on the new computer, but do not start it.
2. Copy the key files you exported: `bugaboo keys import <folder>`. It never replaces a key that the data of an installation needs.
3. Restore the backup: `bugaboo restore <the backup file> --replay-list <folder>/replay.list`. The replay list beside the exported keys is read together with the one in the backup, so nothing that was taken back after the backup comes back.
4. Start Bugaboo.

## The replay list

Some things must stay done even after a restore of an older backup: a person who was removed or switched off, and a key, the access of an AI app or a passkey that was taken back. Bugaboo writes each of them to the replay list in the data folder, and keeps a copy in every backup and beside the exported keys. A restore reads all the copies it can find and does again every line that is newer than the backup.

## How much room a backup takes

A backup is about as big as the database and the attached files together. While the built-in database is copied, the file it writes its newest changes into grows by the changes saved during the copy, and shrinks again after it.
