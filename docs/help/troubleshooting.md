# Troubleshooting
* The window content is way too big and goes off screen.
  * **Linux:** Try setting the `WINIT_X11_SCALE_FACTOR` environment variable to `1`.
    Flatpak installs will have this set automatically.
* The file/folder picker doesn't work.
  * **Steam Deck:** Use desktop mode instead of game mode.
  * **Flatpak:** The `DISPLAY` environment variable may not be getting passed through to the container.
    This has been observed on GNOME systems.
    Try running `flatpak run --nosocket=fallback-x11 --socket=x11 com.github.mtkennerly.ludusavi`.
* On Windows 11, when I open the GUI, a console window also stays open.
  * This is a limitation of the new Windows Terminal app (https://github.com/microsoft/terminal/issues/14416).
    It should be fixed once Windows Terminal v1.17 is released.
    In the meantime, you can work around it by opening Windows Terminal from the Start Menu,
    opening its settings, and changing the "default terminal application" to "Windows Console Host".
* The GUI won't launch.
  * There may be an issue with your graphics drivers/support.
    Try using the software renderer instead by setting the `ICED_BACKEND` environment variable to `tiny-skia`.
  * Try forcing Ludusavi to use your dedicated GPU instead of the integrated graphics.
    One way to do this is by setting the `WGPU_POWER_PREF` environment variable to `high`.
    Alternatively, on Windows 11, go to: Settings app -> System -> Display -> Graphics.
  * You can try prioritizing different hardware renderers
    by setting the `WGPU_BACKEND` environment variable to `dx12`, `vulkan`, `metal`, or `gl`.
  * **Flatpak:** You can try forcing X11 instead of Wayland:
    `flatpak run --nosocket=wayland --socket=x11 com.github.mtkennerly.ludusavi`
* On Windows, I can't back up really long folder/file paths.
  * Ludusavi supports long paths,
    but you also need to enable that feature in Windows itself:
    https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation?tabs=registry#registry-setting-to-enable-long-paths
* When I try to restore backups, I get this error: `Access is denied. (os error 5)`
  * On Windows, Ludusavi may not be able to create folders directly inside of `C:/Program Files`.
    Most commonly, this would be for Steam or other launchers installed in that folder.
    You should reinstall the launchers first (so the parent folders are created),
    then restore the backups.
  * On Windows, Ludusavi cannot create folders directly inside of `C:/Users`.
    If your backups use a different username,
    then you should create a [redirect](/docs/help/redirects.md)
    to restore from your old username to your new username.
  * In general, make sure that your system account has permission to modify the target folders.

## Environment variables on Windows
Some of the instructions above mention setting environment variables.
If you're using Windows and not familiar with how to do this,
you can follow these instructions:

* Open the Start Menu,
  search for `edit the system environment variables`,
  and select the matching result.
* In the new window, click the `environment variables...` button.
* In the upper `user variables` section, click the `new...` button,
  then enter the variable name and value.
  If the variable already exists, select it and click `edit...`.
