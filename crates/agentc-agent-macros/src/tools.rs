// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Error, ItemFn,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

struct ToolArgs {
    /// Optional override for the tool name. Defaults to the function name.
    name: Option<String>,
    /// The capabilities required to invoke this tool.
    capabilities: Vec<String>,
}

impl ToolArgs {
    fn from_attr(attr: TokenStream) -> Self {
        if attr.is_empty() {
            return Self { name: None, capabilities: Vec::new() };
        }

        syn::parse::<ToolArgs>(attr).unwrap_or_else(|e| panic!("Invalid #[tool] arguments: {}", e))
    }
}

impl Parse for ToolArgs {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let mut name = None;
        let mut capabilities = Vec::new();

        let args = Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated(input)?;

        for arg in args {
            match arg {
                syn::Meta::NameValue(nv) if nv.path.is_ident("name") => {
                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = nv.value {
                        name = Some(s.value());
                    }
                }
                syn::Meta::NameValue(nv) if nv.path.is_ident("capabilities") => {
                    if let syn::Expr::Array(arr) = nv.value {
                        for elem in arr.elems {
                            if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = elem
                            {
                                capabilities.push(s.value());
                            }
                        }
                    }
                }
                _ => panic!(
                    "Unknown #[tool] argument: {}",
                    arg.path()
                        .get_ident()
                        .map_or_else(|| "unknown".to_string(), |id| id.to_string())
                ),
            }
        }

        Ok(Self { name, capabilities })
    }
}

fn extract_doc(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| {
            if let syn::Meta::NameValue(nv) = &a.meta {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                    Some(s.value().trim().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extracts the output type information from the function's return type.
///
/// Returns `(output_type, state_update_type, already_wrapped)` where:
/// - `output_type` is the `O` in `ToolOutput<O, S>` or the plain return type
/// - `state_update_type` is `Some(S)` if explicitly provided, `None` if absent (implies `()`)
/// - `already_wrapped` is `true` if the return type is `ToolOutput<O, ...>`
fn extract_ok_type(ty: &syn::Type) -> Option<(syn::Type, Option<syn::Type>, bool)> {
    let syn::Type::Path(tp) = ty else { return None };
    let seg = tp.path.segments.last()?;

    if seg.ident != "Result" {
        return None;
    }

    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return None;
    };

    let inner = if let syn::GenericArgument::Type(t) = args.args.first()? {
        t
    } else {
        return None;
    };

    // Check if inner is TypedToolOutput<O> or TypedToolOutput<O, S>
    if let syn::Type::Path(inner_tp) = inner {
        let last_seg = inner_tp.path.segments.last()?;

        if last_seg.ident == "TypedToolOutput"
            && let syn::PathArguments::AngleBracketed(inner_args) = &last_seg.arguments {
                let mut type_args = inner_args.args.iter().filter_map(|a| {
                    if let syn::GenericArgument::Type(t) = a {
                        Some(t.clone())
                    } else {
                        None
                    }
                });

                let output_type = type_args.next()?;
                let state_update_type = type_args.next(); // Some(S) or None

                return Some((output_type, state_update_type, true));
            }
    }

    // Plain return type, not wrapped in TypedToolOutput
    Some((inner.clone(), None, false))
}

/// Whether the user's function takes `TypedToolInput<I>`, `TypedToolInput<I, S>`, or bare `I`.
///
/// Used by [`tool_impl`] to determine the inner `Input` and `State` associated
/// types and how to forward arguments from the generated `execute` body.
#[allow(clippy::large_enum_variant)]
enum ToolInputStyle {
    /// `fn my_tool(input: TypedToolInput<I>)` - pass `input` directly, no state.
    Wrapped(syn::Type),
    /// `fn my_tool(input: TypedToolInput<I, S>)` - pass `input` directly, with state type `S`.
    WrappedWithState(syn::Type, syn::Type),
    /// `fn my_tool(input: I)` - pass `input.args`, no state.
    Bare(syn::Type),
}

impl ToolInputStyle {
    fn from_ty(ty: &syn::Type) -> Self {
        let syn::Type::Path(tp) = ty else {
            return Self::Bare(ty.clone());
        };
        let Some(seg) = tp.path.segments.last() else {
            return Self::Bare(ty.clone());
        };
        if seg.ident != "TypedToolInput" {
            return Self::Bare(ty.clone());
        }
        let syn::PathArguments::AngleBracketed(ref ab) = seg.arguments else {
            return Self::Bare(ty.clone());
        };
        let mut type_args = ab.args.iter().filter_map(|a| {
            if let syn::GenericArgument::Type(t) = a {
                Some(t.clone())
            } else {
                None
            }
        });
        let input_type = match type_args.next() {
            Some(t) => t,
            None => return Self::Bare(ty.clone()),
        };
        match type_args.next() {
            Some(state_type) => Self::WrappedWithState(input_type, state_type),
            None => Self::Wrapped(input_type),
        }
    }

    fn input_type(&self) -> &syn::Type {
        match self {
            Self::Wrapped(t) | Self::WrappedWithState(t, _) | Self::Bare(t) => t,
        }
    }

    fn state_type(&self) -> Option<&syn::Type> {
        match self {
            Self::WrappedWithState(_, s) => Some(s),
            _ => None,
        }
    }
}

pub fn tool_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let args = ToolArgs::from_attr(attr);

    let fn_name = &func.sig.ident;
    let tool_name = args
        .name
        .unwrap_or_else(|| fn_name.to_string());
    let description = extract_doc(&func.attrs);

    let capabilities_fn = if args.capabilities.is_empty() {
        quote! { ::agentc_agent::types::capability::CapabilitySet::empty() }
    } else {
        let cap_strings = &args.capabilities;
        quote! {
            ::agentc_agent::types::capability::CapabilitySet::from([
                #(#cap_strings),*
            ])
        }
    };

    let input_style = match func.sig.inputs.iter().find_map(|arg| {
        if let syn::FnArg::Typed(pat) = arg {
            Some(&*pat.ty)
        } else {
            None
        }
    }) {
        Some(ty) => ToolInputStyle::from_ty(ty),
        None => {
            return syn::Error::new_spanned(
                &func.sig,
                "#[tool] function must have an input parameter",
            )
            .to_compile_error()
            .into();
        }
    };
    let input_type = input_style.input_type();
    let state_type = input_style.state_type();

    let output_result = match &func.sig.output {
        syn::ReturnType::Type(_, ty) => match extract_ok_type(ty) {
            Some(result) => Ok(result),
            None => Err(syn::Error::new_spanned(
                ty,
                "#[tool] function must return Result<O, E>, Result<ToolOutput<O>, E>, or Result<ToolOutput<O, S>, E>",
            )
            .to_compile_error()),
        },
        _ => Err(syn::Error::new_spanned(
            &func.sig,
            "#[tool] function must return Result<O, E>, Result<ToolOutput<O>, E>, or Result<ToolOutput<O, S>, E>",
        )
        .to_compile_error()),
    };

    let (output_type, state_update_type, already_wrapped) = match output_result {
        Ok(triple) => triple,
        Err(e) => return e.into(),
    };

    let struct_name = quote::format_ident!(
        "{}Tool",
        fn_name
            .to_string()
            .split('_')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>()
    );

    let fn_arg = match input_style {
        ToolInputStyle::Wrapped(_) | ToolInputStyle::WrappedWithState(_, _) => quote! { input },
        ToolInputStyle::Bare(_) => quote! { input.args },
    };

    let execute_body = if already_wrapped {
        quote! { #fn_name(#fn_arg).await.map_err(Into::into) }
    } else {
        quote! {
            #fn_name(#fn_arg).await
                .map(::agentc_agent::tools::types::TypedToolOutput::ok)
                .map_err(Into::into)
        }
    };

    let expanded = match (state_type, state_update_type) {
        (None, None) => {
            // No state, no explicit StateUpdate - works with any S: GraphState
            quote! {
                #func

                struct #struct_name;

                #[::async_trait::async_trait]
                impl<__S> ::agentc_agent::tools::traits::TypedTool<__S> for #struct_name
                where
                    __S: ::agentc_agent::graph::state::GraphState + 'static,
                {
                    type Input = #input_type;
                    type Output = #output_type;
                    type State = ();
                    type StateUpdate = ();

                    fn name(&self) -> &str { #tool_name }
                    fn description(&self) -> &str { #description }
                    fn capabilities(&self) -> CapabilitySet { #capabilities_fn }

                    async fn execute(
                        &self,
                        input: ::agentc_agent::tools::types::TypedToolInput<Self::Input>,
                    ) -> Result<
                        ::agentc_agent::tools::types::TypedToolOutput<Self::Output, ()>,
                        ::agentc_agent::tools::errors::ToolError,
                    > {
                        #execute_body
                    }
                }
            }
        }
        (None, Some(su_type)) => {
            // No state, explicit StateUpdate - works with any S where su_type: IntoStateUpdate<S::Update>
            quote! {
                #func

                struct #struct_name;

                #[::async_trait::async_trait]
                impl<__S> ::agentc_agent::tools::traits::TypedTool<__S> for #struct_name
                where
                    __S: ::agentc_agent::graph::state::GraphState + 'static,
                    #su_type: ::agentc_agent::graph::state::IntoStateUpdate<__S::Update> + Send,
                {
                    type Input = #input_type;
                    type Output = #output_type;
                    type State = ();
                    type StateUpdate = #su_type;

                    fn name(&self) -> &str { #tool_name }
                    fn description(&self) -> &str { #description }
                    fn capabilities(&self) -> CapabilitySet { #capabilities_fn }

                    async fn execute(
                        &self,
                        input: ::agentc_agent::tools::types::TypedToolInput<Self::Input>,
                    ) -> Result<
                        ::agentc_agent::tools::types::TypedToolOutput<Self::Output, #su_type>,
                        ::agentc_agent::tools::errors::ToolError,
                    > {
                        #execute_body
                    }
                }
            }
        }
        (Some(s_type), None) => {
            // State requested, no explicit StateUpdate - works with any S where s_type: FromState<S>
            quote! {
                #func

                struct #struct_name;

                #[::async_trait::async_trait]
                impl<__S> ::agentc_agent::tools::traits::TypedTool<__S> for #struct_name
                where
                    __S: ::agentc_agent::graph::state::GraphState + 'static,
                    #s_type: ::agentc_agent::graph::state::FromState<__S> + Send,
                {
                    type Input = #input_type;
                    type Output = #output_type;
                    type State = #s_type;
                    type StateUpdate = ();

                    fn name(&self) -> &str { #tool_name }
                    fn description(&self) -> &str { #description }
                    fn capabilities(&self) -> CapabilitySet { #capabilities_fn }

                    async fn execute(
                        &self,
                        input: ::agentc_agent::tools::types::TypedToolInput<Self::Input, #s_type>,
                    ) -> Result<
                        ::agentc_agent::tools::types::TypedToolOutput<Self::Output, ()>,
                        ::agentc_agent::tools::errors::ToolError,
                    > {
                        #execute_body
                    }
                }
            }
        }
        (Some(s_type), Some(su_type)) => {
            // State requested and explicit StateUpdate
            quote! {
                #func

                struct #struct_name;

                #[::async_trait::async_trait]
                impl<__S> ::agentc_agent::tools::traits::TypedTool<__S> for #struct_name
                where
                    __S: ::agentc_agent::graph::state::GraphState + 'static,
                    #s_type: ::agentc_agent::graph::state::FromState<__S> + Send,
                    #su_type: ::agentc_agent::graph::state::IntoStateUpdate<__S::Update> + Send,
                {
                    type Input = #input_type;
                    type Output = #output_type;
                    type State = #s_type;
                    type StateUpdate = #su_type;

                    fn name(&self) -> &str { #tool_name }
                    fn description(&self) -> &str { #description }
                    fn capabilities(&self) -> CapabilitySet { #capabilities_fn }

                    async fn execute(
                        &self,
                        input: ::agentc_agent::tools::types::TypedToolInput<Self::Input, #s_type>,
                    ) -> Result<
                        ::agentc_agent::tools::types::TypedToolOutput<Self::Output, #su_type>,
                        ::agentc_agent::tools::errors::ToolError,
                    > {
                        #execute_body
                    }
                }
            }
        }
    };

    TokenStream::from(expanded)
}
