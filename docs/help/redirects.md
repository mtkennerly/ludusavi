# Redirects
You can use redirects to back up or restore to a different location than the original file.
These are listed on the "other" screen, where you can click the plus button to add more
and then enter both the old location (source) and new location (target).

There are multiple types of redirects:

* `Backup`: Applies only for backup mode.
* `Restore`: Applies only for restore mode.
* `Bidirectional`: Uses source -> target in backup mode and target -> source in restore mode.

For example:

* Let's say you backed up some saves from `C:/Games`, but then you decided to move it to `D:/Games`.
  You could create a restore redirect with `C:/Games` as the source and `D:/Games` as the target.
* Let's say you play on two computers with different usernames ("A" and "B"),
  but you know that the saves are otherwise the same,
  so you'd like them to share backups.
  You could create two bidirectional redirects:

  * On Computer A, set source to `C:/Users/A` and target to `C:/Users/main`
  * On computer B, set source to `C:/Users/B` and target to `C:/Users/main`

  Both computers' backups would reference the fake user "main",
  but then they would be restored to the original location for that computer.

Tip: As you're editing your redirects, try running a preview and expanding some
games' file lists. This will show you what effect your redirects
will have when you perform the restore for real.
