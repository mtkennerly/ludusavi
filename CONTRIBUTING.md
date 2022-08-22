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
* Linting:
  * `cargo fmt`
  * `cargo clippy --tests -- -D warnings`
* Activate pre-commit hooks (requires Python):
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
cargo install cargo-lichking

# Verified with commit ba58a5c44ccb7d2e0ca0238d833d17de17c2b53b:
curl -o /c/opt/flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
pip install aiohttp toml
```

#### Generate the license bundle:
```bash
rm -rf dist
mkdir dist
export VERSION=$(cargo pkgid | cut -d# -f2 | cut -d: -f2)
cargo lichking bundle --file dist/ludusavi-v0.11.0-legal.txt
sed -i -E 's/C:\\Users\\[^\\]+/~/g' dist/ludusavi-v$VERSION-legal.txt
```

#### Generate Flatpak sources:
```bash
python /c/opt/flatpak-cargo-generator.py Cargo.lock -o dist/generated-sources.json
```
