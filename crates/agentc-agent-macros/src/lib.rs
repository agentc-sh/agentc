// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

#[allow(unused_extern_crates)]
extern crate self as agentc_agent_macros;

mod tools;

use proc_macro::TokenStream;

/// Attribute macro for defining a tool function. This macro will generate the necessary boilerplate
/// to register the function as a tool in the agentc framework, including handling capabilities and input/output types.
///
/// # Example
///
/// ```rust,ignore
/// use serde::{Serialize, Deserialize};
/// use agentc_agent::tools::macros::tool;
/// use agentc_agent::tools::types::ToolOutput;
/// use agentc_agent::tools::errors::ToolError;
///
/// #[derive(Serialize, Deserialize)]
/// struct AdderInput {
///     a: i32,
///     b: i32,
/// }
///
/// #[derive(Serialize, Deserialize)]
/// struct AdderOutput {
///    result: i32,
/// }
///
/// #[tool]
/// async fn adder(input: AdderInput) -> Result<ToolOutput<AdderOutput>, ToolError> {
///     Ok(ToolOutput::ok(AdderOutput { result: input.a + input.b }))
/// }
///
/// #[derive(Serialize, Deserialize)]
/// struct SubtracterInput {
///     a: i32,
///     b: i32,
/// }
///
/// #[derive(Serialize, Deserialize)]
/// struct SubtracterOutput {
///     result: i32,
/// }
///
/// #[tool(capabilities = ["math::subtract"])]
/// async fn subtracter(input: SubtracterInput) -> Result<SubtracterOutput, ToolError> {
///     Ok(SubtracterOutput { result: input.a - input.b })
/// }
///
/// #[derive(Serialize, Deserialize)]
/// struct DividerInput {
///    a: i32,
///    b: i32,
/// }
///
/// #[derive(Serialize, Deserialize)]
/// struct DividerOutput {
///     result: i32,
/// }
///
/// #[tool(name = "divide_op", capabilities = ["math::divide"])]
/// async fn divider(input: DividerInput) -> Result<DividerOutput, ToolError> {
///     if input.b == 0 {
///         return Err(ToolError::execution_error("divide_op", "division by zero"));
///     }
///
///     Ok(DividerOutput { result: input.a / input.b })
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    tools::tool_impl(attr, item)
}
