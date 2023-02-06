// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::LitStr;

use crate::errors::Errors;
use crate::parse_attrs::{Description, FieldKind, TypeAttrs};
use crate::{Optionality, StructField};

/// Returns a `TokenStream` generating an `argp::help::Help` instance.
///
/// Note: `fields` entries with `is_subcommand.is_some()` will be ignored
/// in favor of the `subcommand` argument.
pub(crate) fn inst_help(
    errors: &Errors,
    ty_attrs: &TypeAttrs,
    fields: &[StructField<'_>],
    subcommand: Option<&StructField<'_>>,
) -> TokenStream {
    let positionals = fields
        .iter()
        .filter(|f| f.kind == FieldKind::Positional && !f.attrs.hidden_help)
        .map(positional_info);

    let options = fields
        .iter()
        .filter(|f| f.long_name.is_some() && !f.attrs.hidden_help)
        .map(|field| option_info(errors, field));

    let commands = if let Some(subcommand) = subcommand {
        let subcommand_ty = subcommand.ty_without_wrapper;

        let mut usage = String::new();
        if !subcommand.optionality.is_required() {
            usage.push('[');
        }
        usage.push_str("<command>");
        if !subcommand.optionality.is_required() {
            usage.push(']');
        }
        usage.push_str(" [<args>]");

        quote! {
            ::std::option::Option::Some(::argp::help::HelpCommands {
                usage: #usage,
                subcommands: <#subcommand_ty as ::argp::SubCommands>::COMMANDS,
                dynamic_subcommands: <#subcommand_ty as ::argp::SubCommands>::dynamic_commands,
            })
        }
    } else {
        quote! { ::std::option::Option::None }
    };

    let description = require_description(errors, Span::call_site(), &ty_attrs.description, "type");
    let footer = ty_attrs
        .footer
        .iter()
        .map(LitStr::value)
        .collect::<Vec<_>>()
        .join("\n\n");

    quote! {
        ::argp::help::Help {
            description: #description,
            positionals: &[ #( #positionals, )* ],
            options: &[ #( #options, )* ],
            commands: #commands,
            footer: #footer,
        }
    }
}

/// Generates an usage string for the given positional argument
/// (e.g. `[<foo>...]`).
fn positional_usage(field: &StructField<'_>) -> String {
    let mut out = String::new();

    if !field.optionality.is_required() {
        out.push('[');
    }
    if field.attrs.greedy.is_none() {
        out.push('<');
    }

    out.push_str(&field.positional_arg_name());

    if field.optionality == Optionality::Repeating {
        out.push_str("...");
    }
    if field.attrs.greedy.is_none() {
        out.push('>');
    }
    if !field.optionality.is_required() {
        out.push(']');
    }

    out
}

/// Formats an usage word for the given option (e.g. `[-f <foo>]`). This
/// function must only be called on options (things with `long_name.is_some()`)
fn option_usage(field: &StructField<'_>) -> String {
    let mut out = String::new();

    // bookend with `[` and `]` if optional
    if !field.optionality.is_required() {
        out.push('[');
    }

    let long_name = field
        .long_name
        .as_ref()
        .expect("missing long name for option");

    if let Some(short) = field.attrs.short.as_ref() {
        out.push('-');
        out.push(short.value());
    } else {
        out.push_str(long_name);
    }

    match field.kind {
        FieldKind::SubCommand | FieldKind::Positional => unreachable!(), // don't have long_name
        FieldKind::Switch => {}
        FieldKind::Option => {
            out.push_str(" <");
            if let Some(arg_name) = &field.attrs.arg_name {
                out.push_str(&arg_name.value());
            } else {
                out.push_str(long_name.trim_start_matches("--"));
            }
            if field.optionality == Optionality::Repeating {
                out.push_str("...");
            }
            out.push('>');
        }
    }

    if !field.optionality.is_required() {
        out.push(']');
    }

    out
}

// TODO(cramertj) make it so this is only called at least once per object so
// as to avoid creating multiple errors.
pub fn require_description(
    errors: &Errors,
    err_span: Span,
    desc: &Option<Description>,
    kind: &str, // the thing being described ("type" or "field"),
) -> String {
    desc.as_ref()
        .map(|d| d.content.value().trim().to_owned())
        .unwrap_or_else(|| {
            errors.err_span(
                err_span,
                &format!(
                    "#[derive(FromArgs)] {} with no description.
Add a doc comment or an `#[argp(description = \"...\")]` attribute.",
                    kind
                ),
            );
            "".to_string()
        })
}

fn positional_info(field: &StructField<'_>) -> TokenStream {
    let usage = positional_usage(field);

    let mut field_name = String::new();
    let mut description = String::new();

    // See explanation in the argp module.
    if field.attrs.greedy.is_none() {
        field_name = field.positional_arg_name();

        if let Some(desc) = &field.attrs.description {
            description = desc.content.value().trim().to_owned()
        }
    }

    quote! {
        ::argp::help::OptionArgInfo {
            usage: #usage,
            names: #field_name,
            description: #description,
            global: false,
        }
    }
}

fn option_info(errors: &Errors, field: &StructField<'_>) -> TokenStream {
    let usage = option_usage(field);

    let short = field.attrs.short.as_ref().map(|s| s.value());
    let long_with_leading_dashes = field
        .long_name
        .as_ref()
        .expect("missing long name for option");

    let arg_name = if field.kind == FieldKind::Option {
        Some(field.positional_arg_name())
    } else {
        None
    };

    let mut names = String::new();

    if let Some(short) = short {
        names.push('-');
        names.push(short);
        names.push_str(", ");
    } else {
        //             "-x, "
        names.push_str("    ");
    }
    names.push_str(long_with_leading_dashes);

    if let Some(arg_name) = arg_name {
        names.push_str(" <");
        names.push_str(&arg_name);
        names.push('>');
    }

    let description =
        require_description(errors, field.name.span(), &field.attrs.description, "field");

    let global = field.attrs.global;

    quote! {
        ::argp::help::OptionArgInfo {
            usage: #usage,
            names: #names,
            description: #description,
            global: #global,
        }
    }
}
