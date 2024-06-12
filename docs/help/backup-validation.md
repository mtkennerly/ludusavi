# Backup validation
On the restore screen, there is a "validate" button that will check the integrity
of the latest backup (full + differential, if any) for each game.
You won't normally need to use this, but it exists for troubleshooting purposes.

Specifically, this checks the following:

* Is mapping.yaml malformed?
* Is any file declared in mapping.yaml, but missing from the actual backup?

If it finds problems, then it will prompt you to create new full backups for the games in question.
At this time, it will not remove the invalid backups, outside of your normal retention settings.
