// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

use {
    crate::{
        errors::Errors,
        parse_attrs::{Description, FieldKind, TypeAttrs},
        Optionality, StructField,
    },
    proc_macro2::{Span, TokenStream},
    quote::quote,
};

const SECTION_SEPARATOR: &str = "\n\n";

/// Returns a `TokenStream` generating a `String` help message.
///
/// Note: `fields` entries with `is_subcommand.is_some()` will be ignored
/// in favor of the `subcommand` argument.
pub(crate) fn help(
    errors: &Errors,
    cmd_name_str_array_ident: syn::Ident,
    ty_attrs: &TypeAttrs,
    fields: &[StructField<'_>],
    subcommand: Option<&StructField<'_>>,
) -> TokenStream {
    let mut format_lit = "Usage: {command_name}".to_string();

    let positional = fields.iter().filter(|f| {
        f.kind == FieldKind::Positional && f.attrs.greedy.is_none() && !f.attrs.hidden_help
    });

    let options = fields.iter().filter(|f| f.long_name.is_some() && !f.attrs.hidden_help);
    for option in options.clone() {
        format_lit.push(' ');
        option_usage(&mut format_lit, option);
    }

    let mut has_positional = false;
    for arg in positional.clone() {
        has_positional = true;
        format_lit.push(' ');
        positional_usage(&mut format_lit, arg);
    }

    let remain = fields.iter().filter(|f| {
        f.kind == FieldKind::Positional && f.attrs.greedy.is_some() && !f.attrs.hidden_help
    });
    for arg in remain {
        format_lit.push(' ');
        positional_usage(&mut format_lit, arg);
    }

    if let Some(subcommand) = subcommand {
        format_lit.push(' ');
        if !subcommand.optionality.is_required() {
            format_lit.push('[');
        }
        format_lit.push_str("<command>");
        if !subcommand.optionality.is_required() {
            format_lit.push(']');
        }
        format_lit.push_str(" [<args>]");
    }

    format_lit.push_str(SECTION_SEPARATOR);

    let description = require_description(errors, Span::call_site(), &ty_attrs.description, "type");
    format_lit.push_str(&description);

    if has_positional {
        format_lit.push_str(SECTION_SEPARATOR);
        format_lit.push_str("Positional Arguments:");
        for arg in positional {
            positional_description(&mut format_lit, arg);
        }
    }

    format_lit.push_str(SECTION_SEPARATOR);
    format_lit.push_str("Options:");
    for option in options {
        option_description(errors, &mut format_lit, option);
    }
    // Also include "help"
    option_description_format(
        &mut format_lit,
        None,
        "-h, --help",
        None,
        "Show this help message and exit",
    );

    let subcommand_calculation;
    let subcommand_format_arg;
    if let Some(subcommand) = subcommand {
        format_lit.push_str(SECTION_SEPARATOR);
        format_lit.push_str("Commands:{subcommands}");
        let subcommand_ty = subcommand.ty_without_wrapper;
        subcommand_format_arg = quote! { subcommands = subcommands };
        subcommand_calculation = quote! {
            let subcommands = argp::print_subcommands(
                <#subcommand_ty as argp::SubCommands>::COMMANDS
                    .iter()
                    .copied()
                    .chain(
                        <#subcommand_ty as argp::SubCommands>::dynamic_commands()
                            .iter()
                            .copied())
            );
        };
    } else {
        subcommand_calculation = TokenStream::new();
        subcommand_format_arg = TokenStream::new()
    }

    for lit in &ty_attrs.footer {
        format_lit.push_str(SECTION_SEPARATOR);
        format_lit.push_str(&lit.value());
    }

    format_lit.push('\n');

    quote! { {
        #subcommand_calculation
        format!(#format_lit, command_name = #cmd_name_str_array_ident.join(" "), #subcommand_format_arg)
    } }
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

/// Describes a positional argument like this:
///  hello       positional argument description
fn positional_description(out: &mut String, field: &StructField<'_>) {
    let field_name = field.positional_arg_name();

    let mut description = String::from("");
    if let Some(desc) = &field.attrs.description {
        description = desc.content.value().trim().to_owned();
    }
    positional_description_format(out, &field_name, &description)
}

fn positional_description_format(out: &mut String, name: &str, description: &str) {
    let info = argp_shared::CommandInfo { name, description };
    argp_shared::write_description(out, &info);
}

/// Describes an option like this:
///  -f, --force       force, ignore minor errors. This description
///                    is so long that it wraps to the next line.
fn option_description(errors: &Errors, out: &mut String, field: &StructField<'_>) {
    let short = field.attrs.short.as_ref().map(|s| s.value());
    let long_with_leading_dashes = field.long_name.as_ref().expect("missing long name for option");
    let description =
        require_description(errors, field.name.span(), &field.attrs.description, "field");

    let arg_name =
        if field.kind == FieldKind::Option { Some(field.positional_arg_name()) } else { None };

    option_description_format(out, short, long_with_leading_dashes, arg_name, &description)
}

fn option_description_format(
    out: &mut String,
    short: Option<char>,
    long_with_leading_dashes: &str,
    arg_name: Option<String>,
    description: &str,
) {
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

    let info = argp_shared::CommandInfo { name: &name, description };
    argp_shared::write_description(out, &info);
}
