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
    On Windows 11, go to: Settings app -> System -> Display -> Graphics.
  * You can try prioritizing different hardware renderers
    by setting the `WGPU_BACKEND` environment variable to `dx12`, `vulkan`, or `metal`.
  * **Flatpak:** You can try forcing X11 instead of Wayland:
    `flatpak run --nosocket=wayland --socket=x11 com.github.mtkennerly.ludusavi`
* On Windows, I can't back up really long folder/file paths.
  * Ludusavi supports long paths,
    but you also need to enable that feature in Windows itself:
    https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation?tabs=registry#registry-setting-to-enable-long-paths

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

## Common Issues and Solutions

### Issue 1: Backup Fails Due to Long File Paths
**Solution:** Ensure that long file paths are enabled in Windows. Refer to the [Microsoft documentation](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation?tabs=registry#registry-setting-to-enable-long-paths) for instructions.

### Issue 2: GUI Not Launching
**Solution:** Check your graphics drivers and support. Try using the software renderer by setting the `ICED_BACKEND` environment variable to `tiny-skia`. You can also try prioritizing different hardware renderers by setting the `WGPU_BACKEND` environment variable to `dx12`, `vulkan`, or `metal`.

### Issue 3: File/Folder Picker Not Working on Steam Deck
**Solution:** Use desktop mode instead of game mode.

### Issue 4: Console Window Stays Open on Windows 11
**Solution:** This is a limitation of the new Windows Terminal app. Refer to the [GitHub issue](https://github.com/microsoft/terminal/issues/14416) for more details. You can work around it by changing the "default terminal application" to "Windows Console Host" in the Windows Terminal settings.

## FAQ

### How do I enable long file paths in Windows?
Refer to the [Microsoft documentation](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation?tabs=registry#registry-setting-to-enable-long-paths) for instructions on enabling long file paths in Windows.

### How do I set environment variables on Windows?
Follow these steps:
1. Open the Start Menu, search for `edit the system environment variables`, and select the matching result.
2. In the new window, click the `environment variables...` button.
3. In the upper `user variables` section, click the `new...` button, then enter the variable name and value. If the variable already exists, select it and click `edit...`.

### What should I do if the GUI won't launch?
Check your graphics drivers and support. Try using the software renderer by setting the `ICED_BACKEND` environment variable to `tiny-skia`. You can also try prioritizing different hardware renderers by setting the `WGPU_BACKEND` environment variable to `dx12`, `vulkan`, or `metal`.

### How do I use the file/folder picker on Steam Deck?
Use desktop mode instead of game mode.

### How do I prevent the console window from staying open on Windows 11?
This is a limitation of the new Windows Terminal app. Refer to the [GitHub issue](https://github.com/microsoft/terminal/issues/14416) for more details. You can work around it by changing the "default terminal application" to "Windows Console Host" in the Windows Terminal settings.
