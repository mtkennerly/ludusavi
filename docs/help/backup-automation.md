# Backup automation
Normally, Ludusavi only runs when you launch it and manually request a backup.
However, it is possible to set up automatic backups that run in the background.
You can do this using any task automation app that can invoke Ludusavi's [command line](/docs/help/command-line.md).

## Windows: Task Scheduler
On Windows, you can use the built-in Task Scheduler app.
This is how to use it on Windows 11:

* Search for `Task Scheduler` in the Start Menu and click to launch it.
* On the right side of Task Scheduler, click `Create Basic Task...`.
* In the popup window, enter the task name (e.g., `Ludusavi`).
  Click `next`.
* Select how often you'd like the backup to occur (e.g., daily).
  Click `next`.
* If you want, you may adjust the exact date and time for the task to start.
  Click `next`.
* Set the task action to `start a program`.
  Click `next`.
* Use the `browse` button to select the full path to your copy of `ludusavi.exe`.

  In the `add arguments` field, enter the following exactly: `backup --force`

  You can leave the `start in` field blank.

  Click `next`.
* On the last screen, click `finish` to create the task.
* You can always view or edit the task on the left side of the main Task Scheduler window,
  in the `Task Scheduler Library` section.

## Linux: `cron`
On Linux, one option is [`cron`](https://en.wikipedia.org/wiki/Cron).
For example, run `crontab -e` in your terminal to begin editing the list of tasks,
then add a daily backup task by adding this line:

```
0 0 * * * /opt/ludusavi backup --force
```

(Use the actual path to your copy of `ludusavi` instead of `/opt/ludusavi`)

## Linux: `systemd` timers
On Linux, another option is [`systemd`](https://en.wikipedia.org/wiki/Systemd) timers.
For example, create two files:

* `~/.config/systemd/user/ludusavi-backup.service`:

  ```
  [Unit]
  Description="Ludusavi backup"

  [Service]
  ExecStart=/opt/ludusavi backup --force
  ```

  (Use the actual path to your copy of `ludusavi` instead of `/opt/ludusavi`)
* `~/.config/systemd/user/ludusavi-backup.timer`:

  ```
  [Unit]
  Description="Ludusavi backup timer"

  [Timer]
  OnCalendar=*-*-* 00:00:00
  Unit=ludusavi-backup.service

  [Install]
  WantedBy=timers.target
  ```

Then run `systemctl --user enable ~/.config/systemd/user/ludusavi-backup.timer` in your terminal.
