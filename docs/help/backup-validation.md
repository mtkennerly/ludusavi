# Backup validation
On the restore screen, there is a "validate" button that will check the integrity
of the latest backup (full + differential, if any) for each game.
You won't normally need to use this, but it exists for troubleshooting purposes.

Specifically, this checks the following:

* Is mapping.yaml malformed?
* Is any file declared in mapping.yaml, but missing from the actual backup?
* For portable Windows/Wine backups, is `pathFormat: semantic-v1` readable,
  and do semantic entries point to files under the `__ludusavi_semantic__/`
  storage namespace?
* Context-aware keys (`__ludusavi_context__/<N>/...`) are validated by
  checking that the storage path exists under the corresponding context
  directory.
* Mixed semantic and legacy absolute keys are supported in `semantic-v1`
  backups. Legacy keys fall back to the standard absolute-path validation.

Ludusavi also preserves `registryFormat` metadata on portable backup chains.
Registry transfer between Windows and Wine is not supported yet, so portable
file backups mark registry data as unsupported instead of trying to translate it.

If it finds problems, then it will prompt you to create new full backups for the games in question.
At this time, it will not remove the invalid backups, outside of your normal retention settings.
