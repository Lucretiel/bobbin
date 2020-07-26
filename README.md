# Bobbin

Bobbin is a small webapp for viewing whole twitter conversations

## Get started

### Build

To create a complete build, simply run `make`. This will build the bobbin server as well as the static frontend assets. The only dependencies here are `yarn` and `cargo` (and basic linux coreutils); everything else is handled from there.

#### Rebuild

The makefile is correctly configured to detect dependencies, so it can be
re-run after any changes and it will do a minimal rebuild of the necessary
changes.

### Run

The bobbin binary will be built in `web/target/debug/bobbin` or `web/target/release/bobbin`. All of its command line options are straightforward; run it with `--help` to get started. You'll need twitter OAuth credentials (consumer key and consumer secret).
