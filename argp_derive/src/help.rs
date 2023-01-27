// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::LitStr;

use crate::errors::Errors;
use crate::parse_attrs::{Description, FieldKind, TypeAttrs};
use crate::{Optionality, StructField};

/// Returns a `TokenStream` generating an `argp::Help` instance.
///
/// Note: `fields` entries with `is_subcommand.is_some()` will be ignored
/// in favor of the `subcommand` argument.
pub(crate) fn inst_help(
    errors: &Errors,
    ty_attrs: &TypeAttrs,
    fields: &[StructField<'_>],
    subcommand: Option<&StructField<'_>>,
) -> TokenStream {
    let mut usage = String::new();

    let positionals = fields.iter().filter(|f| {
        f.kind == FieldKind::Positional && f.attrs.greedy.is_none() && !f.attrs.hidden_help
    });

    let options = fields.iter().filter(|f| f.long_name.is_some() && !f.attrs.hidden_help);
    for option in options.clone() {
        usage.push(' ');
        option_usage(&mut usage, option);
    }

    for arg in positionals.clone() {
        usage.push(' ');
        positional_usage(&mut usage, arg);
    }

    let remain = fields.iter().filter(|f| {
        f.kind == FieldKind::Positional && f.attrs.greedy.is_some() && !f.attrs.hidden_help
    });
    for arg in remain {
        usage.push(' ');
        positional_usage(&mut usage, arg);
    }

    if let Some(subcommand) = subcommand {
        usage.push(' ');
        if !subcommand.optionality.is_required() {
            usage.push('[');
        }
        usage.push_str("<command>");
        if !subcommand.optionality.is_required() {
            usage.push(']');
        }
        usage.push_str(" [<args>]");
    }

    let subcommand_ty = subcommand.map(|s| s.ty_without_wrapper);

    let subcommands = if let Some(subcommand_ty) = subcommand_ty {
        quote! { <#subcommand_ty as argp::SubCommands>::COMMANDS }
    } else {
        quote! { &[] }
    };

    let dynamic_subcommands = if let Some(subcommand_ty) = subcommand_ty {
        quote! { <#subcommand_ty as argp::SubCommands>::dynamic_commands }
    } else {
        quote! { || &[] }
    };

    let description = require_description(errors, Span::call_site(), &ty_attrs.description, "type");
    let positionals_desc = positionals.map(positional_description);
    let options_desc = options.map(|field| option_description(errors, field));
    let footer = ty_attrs.footer.iter().map(LitStr::value).collect::<Vec<_>>().join("\n\n");

    quote! {
        argp::Help {
            usage: #usage,
            description: #description,
            positionals: &[ #( #positionals_desc, )* ],
            options: &[ #( #options_desc, )* ],
            subcommands: #subcommands,
            dynamic_subcommands: #dynamic_subcommands,
            footer: #footer,
        }
    }
}

/// Add positional arguments like `[<foo>...]` to a help format string.
fn positional_usage(out: &mut String, field: &StructField<'_>) {
    if !field.optionality.is_required() {
        out.push('[');
    }
    if field.attrs.greedy.is_none() {
        out.push('<');
    }
    let name = field.positional_arg_name();
    out.push_str(&name);
    if field.optionality == Optionality::Repeating {
        out.push_str("...");
    }
    if field.attrs.greedy.is_none() {
        out.push('>');
    }
    if !field.optionality.is_required() {
        out.push(']');
    }
}

/// Add options like `[-f <foo>]` to a help format string.
/// This function must only be called on options (things with `long_name.is_some()`)
fn option_usage(out: &mut String, field: &StructField<'_>) {
    // bookend with `[` and `]` if optional
    if !field.optionality.is_required() {
        out.push('[');
    }

    let long_name = field.long_name.as_ref().expect("missing long name for option");
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
}

// TODO(cramertj) make it so this is only called at least once per object so
// as to avoid creating multiple errors.
pub fn require_description(
    errors: &Errors,
    err_span: Span,
    desc: &Option<Description>,
    kind: &str, // the thing being described ("type" or "field"),
) -> String {
    desc.as_ref().map(|d| d.content.value().trim().to_owned()).unwrap_or_else(|| {
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

fn positional_description(field: &StructField<'_>) -> TokenStream {
    let field_name = field.positional_arg_name();

    let description = if let Some(desc) = &field.attrs.description {
        desc.content.value().trim().to_owned()
    } else {
        String::new()
    };

    quote! {
        (#field_name, #description)
    }
}

fn option_description(errors: &Errors, field: &StructField<'_>) -> TokenStream {
    let short = field.attrs.short.as_ref().map(|s| s.value());
    let long_with_leading_dashes = field.long_name.as_ref().expect("missing long name for option");
    let description =
        require_description(errors, field.name.span(), &field.attrs.description, "field");

    let arg_name =
        if field.kind == FieldKind::Option { Some(field.positional_arg_name()) } else { None };

    let mut name = String::new();
    if let Some(short) = short {
        name.push('-');
        name.push(short);
        name.push_str(", ");
    }
    name.push_str(long_with_leading_dashes);

    if let Some(arg_name) = arg_name {
        name.push_str(" <");
        name.push_str(&arg_name);
        name.push('>');
    }

    quote! {
        (#name, #description)
    }
}
