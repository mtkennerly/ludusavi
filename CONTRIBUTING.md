## Development
### Prerequisites
Rust 1.44.0 or newer is recommended.

On Linux, you'll need some additional system packages because of clipboard
support:

* Ubuntu: `sudo apt-get install -y gcc libxcb-composite0-dev`

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
