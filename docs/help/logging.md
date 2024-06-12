# Logging
Log files are stored in the config folder (see above).
By default, only warnings and errors are logged,
but you can customize this by setting the `RUST_LOG` environment variable
(e.g., `RUST_LOG=ludusavi=debug`).
The most recent 5 log files are kept, rotating on app launch or when a log reaches 10 MiB.
