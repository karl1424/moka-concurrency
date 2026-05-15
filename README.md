# Checkr

![](inspectify-screenshot.png)

## Architecture

The checkr toolchain is split up into multiple crates:

- `checkr`: Contains the fundamental types and functions for the core analysis analysis and validation of results.
- `checko`: Contains the infrastructure code for running external implementations for the analysis.
- `inspectify`: Contains the application code for displaying analysis external implementations.

Each of the crates have different target audiences: `checko` is meant for admin tasks, such as correcting assignments, running competitions, and for validating submissions in CI. `inspectify` is meant for students to interact with their analysis tool in a user-friendly way. `checkr` is the core analysis implementation, and is purely meant to be used as a dependency in other crates.

To learn more about [checko](./checko/README.md) and [inspectify](./inspectify/README.md), checkout the README in their folders.

## Moka Concurrency

Install dependencies:

1. Install [Rust](https://rust-lang.org/tools/install/)
1. Install [NodeJS](https://nodejs.org/en/download)
1. Install [just](https://github.com/casey/just) using `cargo install just`

Open a bash terminal and run: `just app chip`  
You can now access it at: http://localhost:5173/moka  
The deployment can be accessed at: https://team-concurrent.github.io/moka

To run the tests compile the code with `cargo build --release --bin chip`  
and run the tests with `./target/release/chip group --moka groups.toml reference/ tasks/`
