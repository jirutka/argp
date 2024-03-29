#![recursion_limit = "256"]
// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

//! Implementation of the `FromArgs` and `argp(...)` derive attributes.
//!
//! For more thorough documentation, see the `argp` crate itself.

mod errors;
mod help;
mod markdown;
mod parse_attrs;

use std::collections::HashMap;
use std::iter;
use std::str::FromStr;

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{GenericArgument, LitStr, PathArguments, Type};

use crate::errors::Errors;
use crate::parse_attrs::{check_long_name, FieldAttrs, FieldKind, TypeAttrs};

/// Entrypoint for `#[derive(FromArgs)]`.
#[proc_macro_derive(FromArgs, attributes(argp))]
pub fn argp_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let gen = impl_from_args(&ast);
    gen.into()
}

/// Transform the input into a token stream containing any generated implementations,
/// as well as all errors that occurred.
fn impl_from_args(input: &syn::DeriveInput) -> TokenStream {
    let errors = &Errors::default();
    let type_attrs = &TypeAttrs::parse(errors, input);
    let mut output_tokens = match &input.data {
        syn::Data::Struct(ds) => {
            impl_from_args_struct(errors, &input.ident, type_attrs, &input.generics, ds)
        }
        syn::Data::Enum(de) => {
            impl_from_args_enum(errors, &input.ident, type_attrs, &input.generics, de)
        }
        syn::Data::Union(_) => {
            errors.err(input, "`#[derive(FromArgs)]` cannot be applied to unions");
            TokenStream::new()
        }
    };
    errors.to_tokens(&mut output_tokens);
    output_tokens
}

/// The kind of optionality a parameter has.
enum Optionality {
    None,
    Defaulted(TokenStream),
    Optional,
    Repeating,
}

impl PartialEq<Optionality> for Optionality {
    fn eq(&self, other: &Optionality) -> bool {
        use Optionality::*;
        // NB: (Defaulted, Defaulted) can't contain the same token streams
        matches!((self, other), (Optional, Optional) | (Repeating, Repeating))
    }
}

impl Optionality {
    /// Whether or not this is `Optionality::None`
    fn is_required(&self) -> bool {
        matches!(self, Optionality::None)
    }
}

/// A field of a `#![derive(FromArgs)]` struct with attributes and some other
/// notable metadata appended.
struct StructField<'a> {
    /// The original parsed field
    field: &'a syn::Field,
    /// The parsed attributes of the field
    attrs: FieldAttrs,
    /// The field name. This is contained optionally inside `field`,
    /// but is duplicated non-optionally here to indicate that all field that
    /// have reached this point must have a field name, and it no longer
    /// needs to be unwrapped.
    name: &'a syn::Ident,
    /// Similar to `name` above, this is contained optionally inside `FieldAttrs`,
    /// but here is fully present to indicate that we only have to consider fields
    /// with a valid `kind` at this point.
    kind: FieldKind,
    // If `field.ty` is `Vec<T>` or `Option<T>`, this is `T`, otherwise it's `&field.ty`.
    // This is used to enable consistent parsing code between optional and non-optional
    // keyed and subcommand fields.
    ty_without_wrapper: &'a syn::Type,
    // Whether the field represents an optional value, such as an `Option` subcommand field
    // or an `Option` or `Vec` keyed argument, or if it has a `default`.
    optionality: Optionality,
    // The `--`-prefixed name of the option, if one exists.
    long_name: Option<String>,
}

impl<'a> StructField<'a> {
    /// Attempts to parse a field of a `#[derive(FromArgs)]` struct, pulling out the
    /// fields required for code generation.
    fn new(errors: &Errors, field: &'a syn::Field, attrs: FieldAttrs) -> Option<Self> {
        let name = field.ident.as_ref().expect("missing ident for named field");

        // Ensure that one "kind" is present (switch, option, subcommand, positional)
        let kind = if let Some(field_type) = &attrs.field_type {
            field_type.kind
        } else {
            errors.err(
                field,
                concat!(
                    "Missing `argp` field kind attribute.\n",
                    "Expected one of: `switch`, `option`, `remaining`, `subcommand`, `positional`",
                ),
            );
            return None;
        };

        // Parse out whether a field is optional (`Option` or `Vec`).
        let optionality;
        let ty_without_wrapper;
        match kind {
            FieldKind::Switch => {
                if !ty_expect_switch(errors, &field.ty) {
                    return None;
                }
                optionality = Optionality::Optional;
                ty_without_wrapper = &field.ty;
            }
            FieldKind::Option | FieldKind::Positional => {
                if let Some(default) = &attrs.default {
                    let tokens = match TokenStream::from_str(&default.value()) {
                        Ok(tokens) => tokens,
                        Err(_) => {
                            errors.err(&default, "Invalid tokens: unable to lex `default` value");
                            return None;
                        }
                    };
                    // Set the span of the generated tokens to the string literal
                    let tokens: TokenStream = tokens
                        .into_iter()
                        .map(|mut tree| {
                            tree.set_span(default.span());
                            tree
                        })
                        .collect();
                    optionality = Optionality::Defaulted(tokens);
                    ty_without_wrapper = &field.ty;
                } else {
                    let mut inner = None;
                    optionality = if let Some(x) = ty_inner(&["Option"], &field.ty) {
                        inner = Some(x);
                        Optionality::Optional
                    } else if let Some(x) = ty_inner(&["Vec"], &field.ty) {
                        inner = Some(x);
                        Optionality::Repeating
                    } else {
                        Optionality::None
                    };
                    ty_without_wrapper = inner.unwrap_or(&field.ty);
                }
            }
            FieldKind::SubCommand => {
                let inner = ty_inner(&["Option"], &field.ty);
                optionality = if inner.is_some() {
                    Optionality::Optional
                } else {
                    Optionality::None
                };
                ty_without_wrapper = inner.unwrap_or(&field.ty);
            }
        }

        // Determine the "long" name of options and switches.
        // Defaults to the kebab-case'd field name if `#[argp(long = "...")]` is omitted.
        let long_name = match kind {
            FieldKind::Switch | FieldKind::Option => {
                let long_name = attrs
                    .long
                    .as_ref()
                    .map(syn::LitStr::value)
                    .unwrap_or_else(|| {
                        let kebab_name = to_kebab_case(&name.to_string());
                        check_long_name(errors, name, &kebab_name);
                        kebab_name
                    });
                if long_name == "help" {
                    errors.err(field, "Custom `--help` flags are not supported.");
                }
                let long_name = format!("--{}", long_name);
                Some(long_name)
            }
            FieldKind::SubCommand | FieldKind::Positional => None,
        };

        Some(StructField {
            field,
            attrs,
            kind,
            optionality,
            ty_without_wrapper,
            name,
            long_name,
        })
    }

    pub(crate) fn positional_arg_name(&self) -> String {
        self.attrs
            .arg_name
            .as_ref()
            .map(LitStr::value)
            .unwrap_or_else(|| self.name.to_string().trim_matches('_').to_owned())
    }
}

fn to_kebab_case(s: &str) -> String {
    let words = s.split('_').filter(|word| !word.is_empty());
    let mut res = String::with_capacity(s.len());
    for word in words {
        if !res.is_empty() {
            res.push('-')
        }
        res.push_str(word)
    }
    res
}

/// Implements `FromArgs` and `TopLevelCommand` or `SubCommand` for a `#[derive(FromArgs)]` struct.
fn impl_from_args_struct(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
    ds: &syn::DataStruct,
) -> TokenStream {
    let fields = match &ds.fields {
        syn::Fields::Named(fields) => fields,
        syn::Fields::Unnamed(_) => {
            errors.err(
                &ds.struct_token,
                "`#![derive(FromArgs)]` is not currently supported on tuple structs",
            );
            return TokenStream::new();
        }
        syn::Fields::Unit => {
            errors.err(&ds.struct_token, "#![derive(FromArgs)]` cannot be applied to unit structs");
            return TokenStream::new();
        }
    };

    let fields: Vec<_> = fields
        .named
        .iter()
        .filter_map(|field| {
            let attrs = FieldAttrs::parse(errors, field);
            StructField::new(errors, field, attrs)
        })
        .collect();

    ensure_unique_names(errors, &fields);
    ensure_only_last_positional_is_optional(errors, &fields);
    ensure_only_one_subcommand(errors, &fields);
    ensure_global_only_when_subcommand(errors, &fields);

    let subcommand = fields
        .iter()
        .find(|field| field.kind == FieldKind::SubCommand);

    let impl_span = Span::call_site();

    let from_args_method = impl_from_args_struct_from_args(&fields, subcommand);

    let help_struct = help::inst_help_info(errors, type_attrs, &fields, subcommand);

    let top_or_sub_cmd_impl = top_or_sub_cmd_impl(errors, name, type_attrs, generic_args);

    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    let trait_impl = quote_spanned! { impl_span =>
        #[automatically_derived]
        impl #impl_generics ::argp::FromArgs for #name #ty_generics #where_clause {
            #from_args_method
        }

        #[automatically_derived]
        impl #impl_generics ::argp::CommandHelp for #name #ty_generics #where_clause {
            const HELP: ::argp::help::HelpInfo = #help_struct;
        }

        #top_or_sub_cmd_impl
    };

    trait_impl
}

fn impl_from_args_struct_from_args<'a>(
    fields: &'a [StructField<'a>],
    subcommand: Option<&StructField<'_>>,
) -> TokenStream {
    let init_fields = declare_local_storage_for_from_args_fields(fields);
    let unwrap_fields = unwrap_from_args_fields(fields);
    let positional_fields: Vec<&StructField<'_>> = fields
        .iter()
        .filter(|field| field.kind == FieldKind::Positional)
        .collect();
    let positional_field_idents = positional_fields.iter().map(|field| &field.field.ident);
    let positional_field_names = positional_fields.iter().map(|field| field.name.to_string());
    let last_positional_is_repeating = positional_fields
        .last()
        .map(|field| field.optionality == Optionality::Repeating)
        .unwrap_or(false);
    let last_positional_is_greedy = positional_fields
        .last()
        .map(|field| field.kind == FieldKind::Positional && field.attrs.greedy.is_some())
        .unwrap_or(false);

    let flag_output_table = fields.iter().filter_map(|field| {
        let field_name = &field.field.ident;
        match field.kind {
            FieldKind::Option => {
                Some(quote! { ::argp::parser::ParseStructOption::Value(&mut #field_name) })
            }
            FieldKind::Switch => {
                Some(quote! { ::argp::parser::ParseStructOption::Flag(&mut #field_name) })
            }
            FieldKind::SubCommand | FieldKind::Positional => None,
        }
    });

    let flag_global_table = fields.iter().filter_map(|field| match field.kind {
        FieldKind::Option | FieldKind::Switch => Some(field.attrs.global),
        FieldKind::SubCommand | FieldKind::Positional => None,
    });

    let flag_str_to_output_table_map = flag_str_to_output_table_map_entries(fields);

    let impl_span = Span::call_site();

    let missing_requirements_ident = syn::Ident::new("__missing_requirements", impl_span);

    let append_missing_requirements =
        append_missing_requirements(&missing_requirements_ident, fields);

    let parse_subcommands = if let Some(subcommand) = subcommand {
        let name = subcommand.name;
        let ty = subcommand.ty_without_wrapper;
        quote_spanned! { impl_span =>
            Some(::argp::parser::ParseStructSubCommand {
                subcommands: <#ty as ::argp::SubCommands>::COMMANDS,
                dynamic_subcommands: &<#ty as ::argp::SubCommands>::dynamic_commands(),
                parse_func: &mut |__command, __remaining_args, __parent| {
                    #name = ::std::option::Option::Some(<#ty as ::argp::FromArgs>::_from_args(__command, __remaining_args, __parent)?);
                    ::std::result::Result::Ok(())
                },
            })
        }
    } else {
        quote_spanned! { impl_span => ::std::option::Option::None }
    };

    let method_impl = quote_spanned! { impl_span =>
        fn _from_args(__cmd_name: &[&str], __args: &[&::std::ffi::OsStr], __parent: ::std::option::Option<&mut dyn ::argp::parser::ParseGlobalOptions>)
            -> ::std::result::Result<Self, ::argp::EarlyExit>
        {
            #![allow(clippy::unwrap_in_result)]

            #( #init_fields )*

            ::argp::parser::parse_struct_args(
                __cmd_name,
                __args,
                ::argp::parser::ParseStructOptions {
                    arg_to_slot: &[ #( #flag_str_to_output_table_map ,)* ],
                    slots: &mut [ #( #flag_output_table, )* ],
                    slots_global: &[ #( #flag_global_table, )* ],
                    help: &<Self as argp::CommandHelp>::HELP,
                    parent: __parent,
                },
                ::argp::parser::ParseStructPositionals {
                    positionals: &mut [
                        #(
                            ::argp::parser::ParseStructPositional {
                                name: #positional_field_names,
                                slot: &mut #positional_field_idents as &mut dyn ::argp::parser::ParseValueSlot,
                            },
                        )*
                    ],
                    last_is_repeating: #last_positional_is_repeating,
                    last_is_greedy: #last_positional_is_greedy,
                },
                #parse_subcommands,
                &<Self as ::argp::CommandHelp>::HELP,
            )?;

            let mut #missing_requirements_ident = ::argp::MissingRequirements::default();
            #(
                #append_missing_requirements
            )*
            #missing_requirements_ident.err_on_any()?;

            ::std::result::Result::Ok(Self {
                #( #unwrap_fields, )*
            })
        }
    };

    method_impl
}

/// Ensures that only the last positional arg is non-required.
fn ensure_only_last_positional_is_optional(errors: &Errors, fields: &[StructField<'_>]) {
    let mut first_non_required_span = None;
    for field in fields {
        if field.kind == FieldKind::Positional {
            if let Some(first) = first_non_required_span {
                errors.err_span(
                    first,
                    "Only the last positional argument may be `Option`, `Vec`, or defaulted.",
                );
                errors.err(&field.field, "Later positional argument declared here.");
                return;
            }
            if !field.optionality.is_required() {
                first_non_required_span = Some(field.field.span());
            }
        }
    }
}

/// Ensures that only one short or long name is used.
fn ensure_unique_names(errors: &Errors, fields: &[StructField<'_>]) {
    let mut seen_short_names = HashMap::new();
    let mut seen_long_names = HashMap::new();

    for field in fields {
        if let Some(short_name) = &field.attrs.short {
            let short_name = short_name.value();
            if let Some(first_use_field) = seen_short_names.get(&short_name) {
                errors.err_span_tokens(
                    first_use_field,
                    &format!("The short name of \"-{}\" was already used here.", short_name),
                );
                errors.err_span_tokens(field.field, "Later usage here.");
            }

            seen_short_names.insert(short_name, &field.field);
        }

        if let Some(long_name) = &field.long_name {
            if let Some(first_use_field) = seen_long_names.get(&long_name) {
                errors.err_span_tokens(
                    *first_use_field,
                    &format!("The long name of \"{}\" was already used here.", long_name),
                );
                errors.err_span_tokens(field.field, "Later usage here.");
            }

            seen_long_names.insert(long_name, field.field);
        }
    }
}

/// Ensures that only one field is a `subcommand`.
fn ensure_only_one_subcommand(errors: &Errors, fields: &[StructField<'_>]) {
    let mut subcommands_iter = fields
        .iter()
        .filter(|field| field.kind == FieldKind::SubCommand)
        .fuse();

    let subcommand = subcommands_iter.next();
    for dup_subcommand in subcommands_iter {
        errors.duplicate_attrs("subcommand", subcommand.unwrap().field, dup_subcommand.field);
    }
}

fn ensure_global_only_when_subcommand(errors: &Errors, fields: &[StructField<'_>]) {
    if !fields
        .iter()
        .any(|field| field.kind == FieldKind::SubCommand)
    {
        for field in fields.iter().filter(|field| field.attrs.global) {
            errors.err(
                field.field,
                "`global` may only be used when the struct contains a `#[argp(subcommand)]` field.",
            );
        }
    }
}

/// Implement `::argp::TopLevelCommand` or `::argp::SubCommand` as appropriate.
fn top_or_sub_cmd_impl(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
) -> TokenStream {
    let description =
        help::require_description(errors, name.span(), &type_attrs.description, "type");
    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    if type_attrs.is_subcommand.is_none() {
        // Not a subcommand
        quote! {
            #[automatically_derived]
            impl #impl_generics ::argp::TopLevelCommand for #name #ty_generics #where_clause {}
        }
    } else {
        let empty_str = syn::LitStr::new("", Span::call_site());
        let subcommand_name = type_attrs.name.as_ref().unwrap_or_else(|| {
            errors.err(name, "`#[argp(name = \"...\")]` attribute is required for subcommands");
            &empty_str
        });
        quote! {
            #[automatically_derived]
            impl #impl_generics ::argp::SubCommand for #name #ty_generics #where_clause {
                const COMMAND: &'static ::argp::CommandInfo = &::argp::CommandInfo {
                    name: #subcommand_name,
                    description: #description,
                };
            }
        }
    }
}

/// Declare a local slots to store each field in during parsing.
///
/// Most fields are stored in `Option<FieldType>` locals.
/// `argp(option)` fields are stored in a `ParseValueSlotTy` along with a
/// function that knows how to decode the appropriate value.
fn declare_local_storage_for_from_args_fields<'a>(
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    fields.iter().map(|field| {
        let field_name = &field.field.ident;
        let field_type = &field.ty_without_wrapper;

        // Wrap field types in `Option` if they aren't already `Option` or `Vec`-wrapped.
        let field_slot_type = match field.optionality {
            Optionality::Optional | Optionality::Repeating => (&field.field.ty).into_token_stream(),
            Optionality::None | Optionality::Defaulted(_) => {
                quote! { ::std::option::Option<#field_type> }
            }
        };

        match field.kind {
            FieldKind::Option | FieldKind::Positional => {
                let parse_func = match (&field.attrs.from_os_str_fn, &field.attrs.from_str_fn) {
                    (Some(from_os_str_fn), _) => quote! {
                        |_, value| { #from_os_str_fn(value).map_err(|e| e.to_string()) }
                    },
                    (_, Some(from_str_fn)) => quote! {
                        |_, value| {
                            value.to_str()
                                 .ok_or("not a valid UTF-8 string".to_owned())
                                 .and_then(#from_str_fn)
                                 .map_err(|e| e.to_string())
                        }
                    },
                    (None, None) => quote! {
                        |_, value| { <#field_type as ::argp::FromArgValue>::from_arg_value(value) }
                    }
                };

                quote! {
                    let mut #field_name: ::argp::parser::ParseValueSlotTy<#field_slot_type, #field_type>
                        = ::argp::parser::ParseValueSlotTy {
                            slot: ::std::default::Default::default(),
                            parse_func: #parse_func,
                        };
                }
            }
            FieldKind::SubCommand => {
                quote! { let mut #field_name: #field_slot_type = ::std::option::Option::None; }
            }
            FieldKind::Switch => {
                quote! { let mut #field_name: #field_slot_type = ::argp::parser::Flag::default(); }
            }
        }
    })
}

/// Unwrap non-optional fields and take options out of their tuple slots.
fn unwrap_from_args_fields<'a>(
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    fields.iter().map(|field| {
        let field_name = field.name;
        match field.kind {
            FieldKind::Option | FieldKind::Positional => match &field.optionality {
                Optionality::None => quote! {
                    #field_name: #field_name.slot.unwrap()
                },
                Optionality::Optional | Optionality::Repeating => {
                    quote! { #field_name: #field_name.slot }
                }
                Optionality::Defaulted(tokens) => {
                    quote! {
                        #field_name: #field_name.slot.unwrap_or_else(|| #tokens)
                    }
                }
            },
            FieldKind::Switch => field_name.into_token_stream(),
            FieldKind::SubCommand => match field.optionality {
                Optionality::None => quote! { #field_name: #field_name.unwrap() },
                Optionality::Optional | Optionality::Repeating => field_name.into_token_stream(),
                Optionality::Defaulted(_) => unreachable!(),
            },
        }
    })
}

/// Entries of tokens like `("--some-flag-key", 5)` that map from a flag key string
/// to an index in the output table.
fn flag_str_to_output_table_map_entries<'a>(fields: &'a [StructField<'a>]) -> Vec<TokenStream> {
    let mut flag_str_to_output_table_map = vec![];
    for (i, (field, long_name)) in fields
        .iter()
        .filter_map(|field| field.long_name.as_ref().map(|long_name| (field, long_name)))
        .enumerate()
    {
        if let Some(short) = &field.attrs.short {
            let short = format!("-{}", short.value());
            flag_str_to_output_table_map.push(quote! { (#short, #i) });
        }

        flag_str_to_output_table_map.push(quote! { (#long_name, #i) });
    }
    flag_str_to_output_table_map
}

/// For each non-optional field, add an entry to the `::argp::MissingRequirements`.
fn append_missing_requirements<'a>(
    // missing_requirements_ident
    mri: &syn::Ident,
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    let mri = mri.clone();
    fields
        .iter()
        .filter(|f| f.optionality.is_required())
        .map(move |field| {
            let field_name = field.name;
            match field.kind {
                FieldKind::Switch => unreachable!("switches are always optional"),
                FieldKind::Positional => {
                    let name = field.positional_arg_name();
                    quote! {
                        if #field_name.slot.is_none() {
                            #mri.missing_positional_arg(#name);
                        }
                    }
                }
                FieldKind::Option => {
                    let name = field
                        .long_name
                        .as_ref()
                        .expect("options always have a long name");
                    quote! {
                        if #field_name.slot.is_none() {
                            #mri.missing_option(#name);
                        }
                    }
                }
                FieldKind::SubCommand => {
                    let ty = field.ty_without_wrapper;
                    quote! {
                        if #field_name.is_none() {
                            #mri.missing_subcommands(
                                <#ty as ::argp::SubCommands>::COMMANDS
                                    .iter()
                                    .map(|r| r.name)
                                    .chain(
                                        <#ty as ::argp::SubCommands>::dynamic_commands()
                                            .iter()
                                            .map(|r| r.name)
                                    ),
                            );
                        }
                    }
                }
            }
        })
}

/// Require that a type can be a `switch`.
/// Throws an error for all types except booleans and integers
fn ty_expect_switch(errors: &Errors, ty: &syn::Type) -> bool {
    fn ty_can_be_switch(ty: &syn::Type) -> bool {
        if let syn::Type::Path(path) = ty {
            if path.qself.is_some() {
                return false;
            }
            if path.path.segments.len() != 1 {
                return false;
            }
            let ident = &path.path.segments[0].ident;
            // `Option<bool>` can be used as a `switch`.
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &path.path.segments[0].arguments {
                    if let GenericArgument::Type(Type::Path(p)) = &args.args[0] {
                        if p.path.segments[0].ident == "bool" {
                            return true;
                        }
                    }
                }
            }
            [
                "bool", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128",
            ]
            .iter()
            .any(|path| ident == path)
        } else {
            false
        }
    }

    let res = ty_can_be_switch(ty);
    if !res {
        errors.err(ty, "switches must be of type `bool`, `Option<bool>`, or integer type");
    }
    res
}

/// Returns `Some(T)` if a type is `wrapper_name<T>` for any `wrapper_name` in `wrapper_names`.
fn ty_inner<'a>(wrapper_names: &[&str], ty: &'a syn::Type) -> Option<&'a syn::Type> {
    if let syn::Type::Path(path) = ty {
        if path.qself.is_some() {
            return None;
        }
        // Since we only check the last path segment, it isn't necessarily the case that
        // we're referring to `std::vec::Vec` or `std::option::Option`, but there isn't
        // a fool proof way to check these since name resolution happens after macro expansion,
        // so this is likely "good enough" (so long as people don't have their own types called
        // `Option` or `Vec` that take one generic parameter they're looking to parse).
        let last_segment = path.path.segments.last()?;
        if !wrapper_names.iter().any(|name| last_segment.ident == *name) {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(gen_args) = &last_segment.arguments {
            let generic_arg = gen_args.args.first()?;
            if let syn::GenericArgument::Type(ty) = &generic_arg {
                return Some(ty);
            }
        }
    }
    None
}

/// Implements `FromArgs` and `SubCommands` for a `#![derive(FromArgs)]` enum.
fn impl_from_args_enum(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
    de: &syn::DataEnum,
) -> TokenStream {
    parse_attrs::check_enum_type_attrs(errors, type_attrs, &de.enum_token.span);

    // An enum variant like `<name>(<ty>)`
    struct SubCommandVariant<'a> {
        name: &'a syn::Ident,
        ty: &'a syn::Type,
    }

    let mut dynamic_type_and_variant = None;

    let variants: Vec<SubCommandVariant<'_>> = de
        .variants
        .iter()
        .filter_map(|variant| {
            let name = &variant.ident;
            let ty = enum_only_single_field_unnamed_variants(errors, &variant.fields)?;
            if parse_attrs::VariantAttrs::parse(errors, variant)
                .is_dynamic
                .is_some()
            {
                if dynamic_type_and_variant.is_some() {
                    errors.err(variant, "Only one variant can have the `dynamic` attribute");
                }
                dynamic_type_and_variant = Some((ty, name));
                None
            } else {
                Some(SubCommandVariant { name, ty })
            }
        })
        .collect();

    let variant_ty = variants.iter().map(|x| x.ty).collect::<Vec<_>>();
    let variant_names = variants.iter().map(|x| x.name).collect::<Vec<_>>();

    let from_args_method =
        impl_from_args_enum_from_args(name, &variant_names, &variant_ty, dynamic_type_and_variant);

    let dynamic_commands = dynamic_type_and_variant.as_ref().map(|(dynamic_type, _)| {
        quote! {
            fn dynamic_commands() -> &'static [&'static ::argp::CommandInfo] {
                <#dynamic_type as ::argp::DynamicSubCommand>::commands()
            }
        }
    });

    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    quote! {
        impl #impl_generics ::argp::FromArgs for #name #ty_generics #where_clause {
            #from_args_method
        }

        impl #impl_generics ::argp::SubCommands for #name #ty_generics #where_clause {
            const COMMANDS: &'static [&'static ::argp::CommandInfo] = &[#(
                <#variant_ty as ::argp::SubCommand>::COMMAND,
            )*];

            #dynamic_commands
        }
    }
}

fn impl_from_args_enum_from_args(
    name: &syn::Ident,
    variant_names: &[&syn::Ident],
    variant_ty: &[&Type],
    dynamic_type_and_variant: Option<(&Type, &syn::Ident)>,
) -> TokenStream {
    let name_repeating = iter::repeat(name.clone());

    let dynamic_from_args =
        dynamic_type_and_variant.as_ref().map(|(dynamic_type, dynamic_variant)| {
            quote! {
                if let ::std::option::Option::Some(result) = <#dynamic_type as ::argp::DynamicSubCommand>::try_from_args(
                    command_name, args) {
                    return result.map(#name::#dynamic_variant);
                }
            }
        });

    quote! {
        fn _from_args(
            command_name: &[&str],
            args: &[&::std::ffi::OsStr],
            parent: ::std::option::Option<&mut dyn ::argp::parser::ParseGlobalOptions>,
        ) -> ::std::result::Result<Self, ::argp::EarlyExit> {
            let subcommand_name = if let ::std::option::Option::Some(subcommand_name) = command_name.last() {
                *subcommand_name
            } else {
                return ::std::result::Result::Err(
                    ::argp::EarlyExit::Err(::argp::Error::other("no subcommand name"))
                );
            };

            #(
                if subcommand_name == <#variant_ty as ::argp::SubCommand>::COMMAND.name {
                    return ::std::result::Result::Ok(#name_repeating::#variant_names(
                        <#variant_ty as ::argp::FromArgs>::_from_args(command_name, args, parent)?
                    ));
                }
            )*

            #dynamic_from_args

            ::std::result::Result::Err(
                ::argp::EarlyExit::Err(::argp::Error::other("no subcommand matched"))
            )
        }
    }
}

/// Returns `Some(Bar)` if the field is a single-field unnamed variant like `Foo(Bar)`.
/// Otherwise, generates an error.
fn enum_only_single_field_unnamed_variants<'a>(
    errors: &Errors,
    variant_fields: &'a syn::Fields,
) -> Option<&'a syn::Type> {
    macro_rules! with_enum_suggestion {
        ($help_text:literal) => {
            concat!(
                $help_text,
                "\nInstead, use a variant with a single unnamed field for each subcommand:\n",
                "    enum MyCommandEnum {\n",
                "        SubCommandOne(SubCommandOne),\n",
                "        SubCommandTwo(SubCommandTwo),\n",
                "    }",
            )
        };
    }

    match variant_fields {
        syn::Fields::Named(fields) => {
            errors.err(
                fields,
                with_enum_suggestion!(
                    "`#![derive(FromArgs)]` `enum`s do not support variants with named fields."
                ),
            );
            None
        }
        syn::Fields::Unit => {
            errors.err(
                variant_fields,
                with_enum_suggestion!(
                    "`#![derive(FromArgs)]` does not support `enum`s with no variants."
                ),
            );
            None
        }
        syn::Fields::Unnamed(fields) => {
            if fields.unnamed.len() != 1 {
                errors.err(
                    fields,
                    with_enum_suggestion!(
                        "`#![derive(FromArgs)]` `enum` variants must only contain one field."
                    ),
                );
                None
            } else {
                // `unwrap` is okay because of the length check above.
                let first_field = fields.unnamed.first().unwrap();
                Some(&first_field.ty)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_kebab_case() {
        for (input, expected) in [
            ("", ""),
            ("_", ""),
            ("foo", "foo"),
            ("__foo_", "foo"),
            ("foo_bar", "foo-bar"),
            ("foo__Bar", "foo-Bar"),
            ("foo_bar__baz_", "foo-bar-baz"),
        ] {
            assert_eq!(to_kebab_case(input), expected);
        }
    }
}
