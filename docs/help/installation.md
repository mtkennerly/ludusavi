# Installation
## Requirements
* Ludusavi is available for Windows, Linux, and Mac.
* For the best performance, your system should support one of DirectX, Vulkan, or Metal.
  For other systems, Ludusavi will use a fallback software renderer,
  or you can also activate the software renderer by setting the `ICED_BACKEND` environment variable to `tiny-skia`.

## Methods
You can install Ludusavi one of these ways:

* Download the executable for your operating system from the
  [releases page](https://github.com/mtkennerly/ludusavi/releases).
  It's portable, so you can simply download it and put it anywhere on your system.
  **If you're unsure, choose this option.**

* On Windows, you can use [Winget](https://github.com/microsoft/winget-cli).

  * To install: `winget install -e --id mtkennerly.ludusavi`
  * To update: `winget upgrade -e --id mtkennerly.ludusavi`

* On Windows, you can use [Scoop](https://scoop.sh).

  * To install: `scoop bucket add extras && scoop install ludusavi`
  * To update: `scoop update && scoop update ludusavi`

* For Linux, Ludusavi is available on [Flathub](https://flathub.org/apps/details/com.github.mtkennerly.ludusavi).
  Note that it has limited file system access by default (`~` and `/run/media`).
  If you'd like to enable broader access, [see here](https://github.com/flathub/com.github.mtkennerly.ludusavi/blob/master/README.md).

* If you have [Rust](https://www.rust-lang.org), you can use Cargo.

  * To install or update: `cargo install --locked ludusavi`

  On Linux, this requires the following system packages, or their equivalents
  for your distribution:

  * Ubuntu: `sudo apt-get install -y gcc cmake libx11-dev libxcb-composite0-dev libfreetype6-dev libexpat1-dev libfontconfig1-dev libgtk-3-dev`

## Notes
If you are on Windows:

* When you first run Ludusavi, you may see a popup that says
  "Windows protected your PC",
  because Windows does not recognize the program's publisher.
  Click "more info" and then "run anyway" to start the program.

If you are on Mac:

* When you first run Ludusavi, you may see a popup that says
  "Ludusavi can't be opened because it is from an unidentified developer".
  To allow Ludusavi to run, please refer to [this article](https://support.apple.com/en-us/102445),
  specifically the section on `If you want to open an app [...] from an unidentified developer`.
