// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use agentc_executor_typescript::guestjs::{guest_class, handle::Promise};

use crate::javascript::bindings::{input::ToolInput, output::ToolOutput, tool::Tool};

guest_class! {
    #[guestjs(crate_path = agentc_executor_typescript::guestjs, identity = Tool)]
    pub class GuestTool {
        fn execute(input: ToolInput) -> Promise<ToolOutput>;
    }
}
