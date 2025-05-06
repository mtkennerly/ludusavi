# Cloud backup
Ludusavi integrates with [Rclone](https://rclone.org) to provide cloud backups.
You can configure this on the "other" screen.
Any Rclone remote is supported, but Ludusavi can help you configure some of the more common ones:
Google Drive, OneDrive, Dropbox, Box, FTP servers, SMB servers, and WebDAV servers.
Support is verified for Rclone 1.62.2, but other versions should work as well.

If you turn on automtic synchronization,
then Ludusavi will check if your local and cloud saves are already in sync at the start of a backup.
If so, then any changes will be uploaded once the backup is done.
If they weren't in sync to begin with, then Ludusavi will warn you about the conflict and leave the cloud data alone.
You can perform an upload or download at any time on the "other" screen to resolve such a conflict.

Bear in mind that many factors can affect cloud sync performance,
including network speed, outages on the cloud side, and any limitations of Rclone itself.
You can try setting custom Rclone arguments if you find that it is too slow.
For example, `--fast-list` and/or `--ignore-checksum` can speed things up,
while `--transfers=1` can help to avoid rate-limiting but may slow things down.
The "other" screen has a field to configure custom arguments,
and you can find documentation for them here: https://rclone.org/flags

You can also use other cloud backup tools of your choice,
as long as they can make the storage available as what looks like a normal folder.
For example:

* If you use something like [Google Drive for Desktop](https://www.google.com/drive/download),
  which creates a special drive (`G:`) to stream from/to the cloud,
  then you can set Ludusavi's backup target to a folder in that drive.
* If you use something like [Syncthing](https://syncthing.net),
  which continuously synchronizes a local folder across systems,
  then you can set Ludusavi's backup target to that local folder.
* If you use Rclone's mounting functionality,
  then you can set Ludusavi's backup target to the mount folder.

## Rclone and Flatpak
For Linux users who have installed Ludusavi via Flatpak,
the default Flatpak permissions will keep Ludusavi from seeing your system copy of Rclone.
Therefore, a copy of Rclone is included in the Flatpak environment,
which you can reference as `/app/bin/rclone` in Ludusavi.

If you prefer to use your system copy of Rclone,
one solution is to give Ludusavi host filesystem access
(`flatpak override com.github.mtkennerly.ludusavi --filesystem=host`).
Then, in Ludusavi, you can set the Rclone executable path to
`/var/run/host/usr/bin/rclone`.

You can also configure Ludusavi's list of Rclone arguments to include
`--config /home/<USER>/.config/rclone/rclone.conf`
if you want to share the configuration from your system copy.
