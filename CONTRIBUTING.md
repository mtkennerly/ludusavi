## Development
### Prerequisites
Use the latest version of Rust.

On Linux, you'll need some additional system packages. Refer to the README
for the list.

### Commands
* Run program:
  * `cargo run`
* Run tests:
  * One-time setup:
    * Windows:
      ```
      reg import tests/ludusavi.reg
      cd tests/root3/game5
      mklink /J data-symlink data
      ```
    * Other:
      ```
      cd tests/root3/game5
      ln -s data data-symlink
      ```
  * `cargo test`
* Activate pre-commit hooks (requires Python) to handle formatting/linting:
  ```
  pip install --user pre-commit
  pre-commit install
  ```

### Environment variables
These are optional:

* `LUDUSAVI_VERSION`:
  * If set, shown in the window title instead of the Cargo.toml version.
  * Intended for CI.
* `LUDUSAVI_VARIANT`:
  * If set, shown in the window title in parentheses.
  * Intended for alternative builds, such as using different Iced renderers.

### Icon
The master icon is `assets/icon.kra`, which you can edit using
[Krita](https://krita.org/en) and then export into the other formats.

### Release preparation
Commands assume you are using [Git Bash](https://git-scm.com) on Windows.

#### Dependencies (one-time)
```bash
pip install invoke
cargo install cargo-lichking

# Verified with commit ba58a5c44ccb7d2e0ca0238d833d17de17c2b53b:
curl -o /c/opt/flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
pip install aiohttp toml
```

Also install the Crowdin CLI manually.

#### Process
* Update version in CHANGELOG.md
* Update version in Cargo.toml
* Update version in assets/com.github.mtkennerly.ludusavi.metainfo.xml
  including the `releases` section and the screenshot URL.
* Run `invoke prerelease`
  * If you already updated the translations separately,
    then run `invoke prerelease --no-update-lang`
* Update the translation percentages in src/lang.rs
* Run `cargo build` to update the version in Cargo.lock
* Add the new version to `.github/ISSUE_TEMPLATE/*.yaml`.
* Update the README if necessary for any new features.
  Check for any new content that needs to be uncommented (`<!--`).

#### Publish
Commands assume you've set `VERSION=$(invoke version)`.

* Flatpak:
  * Use fork of https://github.com/flathub/com.github.mtkennerly.ludusavi .
  * From master, create a new branch (`release/v${VERSION}`).
  * Update `com.github.mtkennerly.ludusavi.yaml` to reference the new tag.
  * Replace `generated-sources.json` (new file produced by `invoke prerelease` earlier).
  * Open a pull request.
    * Recommended commit message and PR title:
      `Update for v${VERSION}`
  * After the PR is merged, publish via https://buildbot.flathub.org/#/apps/com.github.mtkennerly.ludusavi .
* winget:
  * Use fork of https://github.com/microsoft/winget-pkgs .
  * From master, create a new branch (`mtkennerly.ludusavi-${VERSION}`).
  * Run `wingetcreate update mtkennerly.ludusavi --version ${VERSION} --urls https://github.com/mtkennerly/ludusavi/releases/download/v${VERSION}/ludusavi-v${VERSION}-win64.zip https://github.com/mtkennerly/ludusavi/releases/download/v${VERSION}/ludusavi-v${VERSION}-win32.zip`
  * In the generated `manifests/m/mtkennerly/ludusavi/${VERSION}/mtkennerly.ludusavi.locale.en-US.yaml` file,
    add the `ReleaseNotes` and `ReleaseNotesUrl` fields:

    ```yaml
    ReleaseNotes: |-
      <copy/paste from CHANGELOG.md>
    ReleaseNotesUrl: https://github.com/mtkennerly/ludusavi/releases/tag/v${VERSION}
    ```
  * Run `winget validate --manifest manifests/m/mtkennerly/ludusavi/${VERSION}`
  * Open a pull request.
    * Recommended commit message and PR title:
      `mtkennerly.ludusavi version ${VERSION}`
