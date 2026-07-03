// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_TAG");

    if let Ok(v) = std::env::var("BUILD_TAG")
        && !v.is_empty()
    {
        println!("cargo:rustc-env=CARGO_PKG_VERSION={}", v);
    }
}
