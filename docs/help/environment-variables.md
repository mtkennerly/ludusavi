# Environment variables
Environment variables can be used to tweak some additional behavior:

* `RUST_LOG`: Configure logging.
  Example: `RUST_LOG=ludusavi=debug`
* `LUDUSAVI_DEBUG`: If this is set to any value,
  then Ludusavi will not detach from the console on Windows in GUI mode.
  It will also print some debug messages in certain cases.
  Example: `LUDUSAVI_DEBUG=1`
* `LUDUSAVI_THREADS`: Overrive the `runtime.threads` value from the config file.
  Example: `LUDUSAVI_THREADS=8`
* `LUDUSAVI_LINUX_APP_ID`: On Linux, this can override Ludusavi's application ID.
  The default is `com.mtkennerly.ludusavi`.
  This should match the corresponding `.desktop` file.
