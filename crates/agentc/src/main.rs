// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[cfg(feature = "cli")]
mod cli;

#[cfg(feature = "cli")]
#[tokio::main]
async fn main() {
    cli::CliArgs::run().await;
}

#[cfg(not(feature = "cli"))]
fn main() {
    println!("This binary is only available with the 'cli' feature enabled.");
}
