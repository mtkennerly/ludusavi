## Development
### Prerequisites
Rust 1.44.0 or newer is recommended.

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
  * Intended for alternative builds, such as for OpenGL support.

### Icon
The master icon is `assets/icon.kra`, which you can edit using
[Krita](https://krita.org/en) and then export into the other formats.
