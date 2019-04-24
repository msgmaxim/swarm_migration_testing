# Swarm Migration Testing
Framework for testing data migration for swarms.

## Building

1. Install *rustup* (https://rustup.rs/) (make sure `cargo` is in `PATH`, e.g. by following *rustup* instructions)
2. `cargo build --release`

## Configuring

In `src/swarm.rs` replace `SERVER_PATH` to point to the storage server executable on your machine. Yes, this is not super user-friendly (yet) (TODO).

## Selecting a test

In `src/main.rs` uncomment a line corresponding to the test to run. Note that only one test per execution is possible at the moment.
The most useful test is arguably `test::test_blocks` which will create "blocks" every second or so with SNode events (registrations, deregistrations, swarm migrations etc.) and send messages to active SNodes on a separate thread.

## Running a test

`cargo run --release` (this will automatically run `cargo build` if necessary)

Upon completion the test will report on missing messages. Sample output from a successful run:

`Test passed! (4104/4104 messages)`

- Note that a `playground` directory will be created with logs from each server instance (i.e. SNode) and their database files.
The log from the testing framework itself will be available in `log/tests.log` (partially printed to stdout).
Both are automatically purged before each run.