// SPDX-License-Identifier: BSD-3-Clause
// SPDX-FileCopyrightText: 2023 Jakub Jirutka <jakub@jirutka.cz>
// SPDX-FileCopyrightText: 2020 Google LLC

use std::fmt;

use proc_macro2::Span;

use crate::errors::Errors;
use crate::markdown;

/// Attributes applied to a field of a `#![derive(FromArgs)]` struct.
#[derive(Default)]
pub struct FieldAttrs {
    pub default: Option<syn::LitStr>,
    pub description: Option<Description>,
    pub from_str_fn: Option<syn::Path>,
    pub from_os_str_fn: Option<syn::Path>,
    pub field_type: Option<FieldType>,
    pub long: Option<syn::LitStr>,
    pub short: Option<syn::LitChar>,
    pub arg_name: Option<syn::LitStr>,
    pub greedy: Option<syn::Path>,
    pub hidden_help: bool,
    pub global: bool,
}

/// The purpose of a particular field on a `#![derive(FromArgs)]` struct.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FieldKind {
    /// Switches are booleans that are set to "true" by passing the flag.
    Switch,
    /// Options are `--key value`. They may be optional (using `Option`),
    /// or repeating (using `Vec`), or required (neither `Option` nor `Vec`)
    Option,
    /// Subcommand fields (of which there can be at most one) refer to enums
    /// containing one of several potential subcommands. They may be optional
    /// (using `Option`) or required (no `Option`).
    SubCommand,
    /// Positional arguments are parsed literally if the input
    /// does not begin with `-` or `--` and is not a subcommand.
    /// They are parsed in declaration order, and only the last positional
    /// argument in a type may be an `Option`, `Vec`, or have a default value.
    Positional,
}

/// The type of a field on a `#![derive(FromArgs)]` struct.
///
/// This is a simple wrapper around `FieldKind` which includes the `syn::Ident`
/// of the attribute containing the field kind.
pub struct FieldType {
    pub kind: FieldKind,
    pub ident: syn::Ident,
}

/// A description of a `#![derive(FromArgs)]` struct.
///
/// Defaults to the docstring if one is present, or `#[argp(description = "...")]`
/// if one is provided.
pub struct Description {
    /// Whether the description was an explicit annotation or whether it was a doc string.
    pub explicit: bool,
    pub lines: Vec<String>,
    pub span: Span,
}

impl FieldAttrs {
    pub fn parse(errors: &Errors, field: &syn::Field) -> Self {
        let mut this = Self::default();

        for attr in &field.attrs {
            if is_doc_attr(attr) {
                parse_attr_doc(errors, attr, &mut this.description);
                continue;
            }

            let ml = if let Some(ml) = argp_attr_to_meta_list(errors, attr) {
                ml
            } else {
                continue;
            };

            for meta in ml {
                let name = meta.path();
                if name.is_ident("arg_name") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_single_string(errors, m, "arg_name", &mut this.arg_name);
                    }
                } else if name.is_ident("default") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_single_string(errors, m, "default", &mut this.default);
                    }
                } else if name.is_ident("description") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_description(errors, m, &mut this.description);
                    }
                } else if name.is_ident("from_str_fn") {
                    if let Some(m) = errors.expect_meta_list(&meta) {
                        parse_attr_fn_path(errors, m, "from_str_fn", &mut this.from_str_fn);
                    }
                } else if name.is_ident("from_os_str_fn") {
                    if let Some(m) = errors.expect_meta_list(&meta) {
                        parse_attr_fn_path(errors, m, "from_os_str_fn", &mut this.from_os_str_fn);
                    }
                } else if name.is_ident("long") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_long(errors, m);
                    }
                } else if name.is_ident("option") {
                    parse_attr_field_type(errors, &meta, FieldKind::Option, &mut this.field_type);
                } else if name.is_ident("short") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_short(errors, m);
                    }
                } else if name.is_ident("subcommand") {
                    parse_attr_field_type(
                        errors,
                        &meta,
                        FieldKind::SubCommand,
                        &mut this.field_type,
                    );
                } else if name.is_ident("switch") {
                    parse_attr_field_type(errors, &meta, FieldKind::Switch, &mut this.field_type);
                } else if name.is_ident("positional") {
                    parse_attr_field_type(
                        errors,
                        &meta,
                        FieldKind::Positional,
                        &mut this.field_type,
                    );
                } else if name.is_ident("greedy") {
                    this.greedy = Some(name.clone());
                } else if name.is_ident("hidden_help") {
                    this.hidden_help = true;
                } else if name.is_ident("global") {
                    this.global = true;
                } else {
                    errors.err(
                        &meta,
                        concat!(
                            "Invalid field-level `argp` attribute\n",
                            "Expected one of: `arg_name`, `default`, `description`, `from_os_str_fn`, ",
                            "`from_str_fn`, `global`, `greedy`, `long`, `option`, `short`, `subcommand`, ",
                            "`switch`, `hidden_help`",
                        ),
                    );
                }
            }
        }

        if let (Some(default), Some(field_type)) = (&this.default, &this.field_type) {
            match field_type.kind {
                FieldKind::Option | FieldKind::Positional => {}
                FieldKind::SubCommand | FieldKind::Switch => errors.err(
                    default,
                    "`default` may only be specified on `#[argp(option)]` \
                     or `#[argp(positional)]` fields",
                ),
            }
        }

        match (&this.greedy, this.field_type.as_ref().map(|f| f.kind)) {
            (Some(_), Some(FieldKind::Positional)) => {}
            (Some(greedy), Some(_)) => errors.err(
                &greedy,
                "`greedy` may only be specified on `#[argp(positional)]` \
                    fields",
            ),
            _ => {}
        }

        if let (Some(field_type), true) = (&this.field_type, this.global) {
            match field_type.kind {
                FieldKind::Option | FieldKind::Switch => {}
                FieldKind::Positional | FieldKind::SubCommand => errors.err(
                    &field,
                    "`global` may only be specified on `#[argp(option)]` \
                     or `#[argp(switch)]` fields",
                ),
            }
        }

        if let (Some(from_str_fn), Some(_)) = (&this.from_str_fn, &this.from_os_str_fn) {
            errors.err(&from_str_fn, "`from_str_fn` and `from_os_str_fn` are mutually exclusive")
        }

        this
    }

    fn parse_attr_long(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "long", &mut self.long);
        let long = self.long.as_ref().unwrap();
        let value = long.value();
        check_long_name(errors, long, &value);
    }

    fn parse_attr_short(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        if let Some(first) = &self.short {
            errors.duplicate_attrs("short", first, m);
        } else if let Some(lit_char) = errors.expect_lit_char(&m.value) {
            self.short = Some(lit_char.clone());
            if !lit_char.value().is_ascii() {
                errors.err(lit_char, "Short names must be ASCII");
            }
        }
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.explicit {
            f.write_str(&self.lines.join("\n"))
        } else {
            let mut buf = String::new();
            for line in &self.lines {
                buf.push_str(&line.replace('\n', "\\n"));
                buf.push('\n');
            }
            f.write_str(&markdown::to_plain_text(&buf))
        }
    }
}

pub(crate) fn check_long_name(errors: &Errors, spanned: &impl syn::spanned::Spanned, value: &str) {
    if !value.is_ascii() {
        errors.err(spanned, "Long names must be ASCII");
    }
    if !value
        .chars()
        .all(|c| c.is_lowercase() || c == '-' || c.is_ascii_digit())
    {
        errors.err(spanned, "Long names must be lowercase");
    }
}

fn parse_attr_fn_path(
    errors: &Errors,
    m: &syn::MetaList,
    attr_name: &str,
    slot: &mut Option<syn::Path>,
) {
    if let Some(first) = slot {
        errors.duplicate_attrs(attr_name, first, m);
    }
    *slot = m.parse_args().map_err(|e| errors.push(e)).ok();
}

fn parse_attr_field_type(
    errors: &Errors,
    meta: &syn::Meta,
    kind: FieldKind,
    slot: &mut Option<FieldType>,
) {
    if let Some(path) = errors.expect_meta_word(meta) {
        if let Some(first) = slot {
            errors.duplicate_attrs("field kind", &first.ident, path);
        } else if let Some(word) = path.get_ident() {
            *slot = Some(FieldType {
                kind,
                ident: word.clone(),
            });
        }
    }
}

// Whether the attribute is one like `#[<name> ...]`
fn is_matching_attr(name: &str, attr: &syn::Attribute) -> bool {
    attr.path().segments.len() == 1 && attr.path().segments[0].ident == name
}

/// Checks for `#[doc ...]`, which is generated by doc comments.
fn is_doc_attr(attr: &syn::Attribute) -> bool {
    is_matching_attr("doc", attr)
}

/// Checks for `#[argp ...]`
fn is_argp_attr(attr: &syn::Attribute) -> bool {
    is_matching_attr("argp", attr)
}

/// Filters out non-`#[argp(...)]` attributes and converts to a sequence of
/// `syn::Meta`.
fn argp_attr_to_meta_list(
    errors: &Errors,
    attr: &syn::Attribute,
) -> Option<impl IntoIterator<Item = syn::Meta>> {
    if !is_argp_attr(attr) {
        return None;
    }
    let meta_list = errors.expect_meta_list(&attr.meta)?;

    meta_list
        .parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .map_err(|e| errors.push(e))
        .ok()
}

/// Represents a `#[derive(FromArgs)]` type's top-level attributes.
#[derive(Default)]
pub struct TypeAttrs {
    pub is_subcommand: Option<syn::Ident>,
    pub name: Option<syn::LitStr>,
    pub description: Option<Description>,
    pub footer: Vec<syn::LitStr>,
}

impl TypeAttrs {
    /// Parse top-level `#[argp(...)]` attributes
    pub fn parse(errors: &Errors, derive_input: &syn::DeriveInput) -> Self {
        let mut this = TypeAttrs::default();

        for attr in &derive_input.attrs {
            if is_doc_attr(attr) {
                parse_attr_doc(errors, attr, &mut this.description);
                continue;
            }

            let ml = if let Some(ml) = argp_attr_to_meta_list(errors, attr) {
                ml
            } else {
                continue;
            };

            for meta in ml {
                let name = meta.path();
                if name.is_ident("description") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_description(errors, m, &mut this.description);
                    }
                } else if name.is_ident("footer") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_multi_string(errors, m, &mut this.footer)
                    }
                } else if name.is_ident("name") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_name(errors, m);
                    }
                } else if name.is_ident("subcommand") {
                    if let Some(ident) = errors.expect_meta_word(&meta).and_then(|p| p.get_ident())
                    {
                        this.parse_attr_subcommand(errors, ident);
                    }
                } else {
                    errors.err(
                        &meta,
                        concat!(
                            "Invalid type-level `argp` attribute\n",
                            "Expected one of: `description`, `footer`, `name`, `note`, `subcommand`",
                        ),
                    );
                }
            }
        }

        this
    }

    fn parse_attr_name(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "name", &mut self.name);
        if let Some(name) = &self.name {
            if name.value() == "help" {
                errors.err(name, "Custom `help` commands are not supported.");
            }
        }
    }

    fn parse_attr_subcommand(&mut self, errors: &Errors, ident: &syn::Ident) {
        if let Some(first) = &self.is_subcommand {
            errors.duplicate_attrs("subcommand", first, ident);
        } else {
            self.is_subcommand = Some(ident.clone());
        }
    }
}

/// Represents an enum variant's attributes.
#[derive(Default)]
pub struct VariantAttrs {
    pub is_dynamic: Option<syn::Path>,
}

impl VariantAttrs {
    /// Parse enum variant `#[argp(...)]` attributes
    pub fn parse(errors: &Errors, variant: &syn::Variant) -> Self {
        let mut this = VariantAttrs::default();

        let fields = match &variant.fields {
            syn::Fields::Named(fields) => Some(&fields.named),
            syn::Fields::Unnamed(fields) => Some(&fields.unnamed),
            syn::Fields::Unit => None,
        };

        for field in fields.into_iter().flatten() {
            for attr in &field.attrs {
                if is_argp_attr(attr) {
                    err_unused_enum_attr(errors, attr);
                }
            }
        }

        for attr in &variant.attrs {
            let ml = if let Some(ml) = argp_attr_to_meta_list(errors, attr) {
                ml
            } else {
                continue;
            };

            for meta in ml {
                let name = meta.path();
                if name.is_ident("dynamic") {
                    if let Some(prev) = this.is_dynamic.as_ref() {
                        errors.duplicate_attrs("dynamic", prev, &meta);
                    } else {
                        this.is_dynamic = errors.expect_meta_word(&meta).cloned();
                    }
                } else {
                    errors.err(
                        &meta,
                        "Invalid variant-level `argp` attribute\n\
                         Variants can only have the #[argp(dynamic)] attribute.",
                    );
                }
            }
        }

        this
    }
}

fn parse_attr_single_string(
    errors: &Errors,
    m: &syn::MetaNameValue,
    name: &str,
    slot: &mut Option<syn::LitStr>,
) {
    if let Some(first) = slot {
        errors.duplicate_attrs(name, first, m);
    } else if let Some(lit_str) = errors.expect_lit_str(&m.value) {
        *slot = Some(lit_str.clone());
    }
}

fn parse_attr_multi_string(errors: &Errors, m: &syn::MetaNameValue, list: &mut Vec<syn::LitStr>) {
    if let Some(lit_str) = errors.expect_lit_str(&m.value) {
        list.push(lit_str.clone());
    }
}

fn parse_attr_doc(errors: &Errors, attr: &syn::Attribute, slot: &mut Option<Description>) {
    let nv = if let Some(nv) = errors.expect_meta_name_value(&attr.meta) {
        nv
    } else {
        return;
    };

    // Don't replace an existing description.
    if slot.as_ref().map(|d| d.explicit).unwrap_or(false) {
        return;
    }

    if let Some(lit_str) = errors.expect_lit_str(&nv.value) {
        if let Some(slot) = slot {
            slot.lines.push(lit_str.value());
        } else {
            *slot = Some(Description {
                explicit: false,
                lines: vec![lit_str.value()],
                span: lit_str.span(),
            });
        };
    }
}

fn parse_attr_description(errors: &Errors, m: &syn::MetaNameValue, slot: &mut Option<Description>) {
    let lit_str = if let Some(lit_str) = errors.expect_lit_str(&m.value) {
        lit_str
    } else {
        return;
    };

    // Don't allow multiple explicit (non doc-comment) descriptions
    if let Some(description) = slot {
        if description.explicit {
            errors.duplicate_attrs("description", &description.span, lit_str);
        }
    }

    *slot = Some(Description {
        explicit: true,
        lines: vec![lit_str.value()],
        span: lit_str.span(),
    });
}

/// Checks that a `#![derive(FromArgs)]` enum has an `#[argp(subcommand)]`
/// attribute and that it does not have any other type-level `#[argp(...)]` attributes.
pub fn check_enum_type_attrs(errors: &Errors, type_attrs: &TypeAttrs, type_span: &Span) {
    let TypeAttrs {
        is_subcommand,
        name,
        description,
        footer,
    } = type_attrs;

    // Ensure that `#[argp(subcommand)]` is present.
    if is_subcommand.is_none() {
        errors.err_span(
            *type_span,
            concat!(
                "`#![derive(FromArgs)]` on `enum`s can only be used to enumerate subcommands.\n",
                "Consider adding `#[argp(subcommand)]` to the `enum` declaration.",
            ),
        );
    }

    // Error on all other type-level attributes.
    if let Some(name) = name {
        err_unused_enum_attr(errors, name);
    }
    if let Some(description) = description {
        if description.explicit {
            err_unused_enum_attr(errors, &description.span);
        }
    }
    if let Some(footer) = footer.first() {
        err_unused_enum_attr(errors, footer);
    }
}

fn err_unused_enum_attr(errors: &Errors, location: &impl syn::spanned::Spanned) {
    errors.err(
        location,
        concat!(
            "Unused `argp` attribute on `#![derive(FromArgs)]` enum. ",
            "Such `enum`s can only be used to dispatch to subcommands, ",
            "and should only contain the #[argp(subcommand)] attribute.",
        ),
    );
}
