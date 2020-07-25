## Development
### Prerequisites
Rust 1.44.0 or newer is recommended.

On Linux, you'll need some additional system packages. Refer to the README
for the list.

### Commands
* Run program:
  * `cargo run`
* Run tests:
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
* `LUDUSAVI_VERSION`:
  * If set, shown in the window title instead of the Cargo.toml version.
  * Intended for CI.
* `LUDUSAVI_VARIANT`:
  * If set, shown in the window title in parentheses.
  * Intended for alternative builds, such as for OpenGL support.

### Registry
On Windows, before running the tests, you need to import `tests/ludusavi.reg`.
