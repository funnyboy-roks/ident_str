//! A macro to convert string literals into identifiers.  The primary use-case is to allow
//! declarative macros to produce identifiers that are unique to each call.  The alternative is to
//! accept each identifier as an argument to the macro call, which gets unweildy with many
//! identifiers.
//!
//! # Usage
//!
//! ```
//! macro_rules! my_macro {
//!     ($name: ident) => {
//!         ident_str::ident_str! {
//!             #name_a = concat!(stringify!($name), "_a"),
//!             #name_b = concat!(stringify!($name), "_b"),
//!             => {
//!                 fn #name_a() {}
//!                 fn #name_b() {}
//!             }
//!         }
//!     };
//! }
//!
//! my_macro!(foo);
//! ```
//!
//! expands to
//!
//! ```
//! fn foo_a() {}
//! fn foo_b() {}
//! ```
//!
//! # Supported Macros
//!
//! This crate takes advantage of [`MacroString`](https://github.com/dtolnay/macro-string/) and
//! supports all macros supported by it.  Those macros are:
//! - `concat!`
//! - `stringify!`
//! - `env!`
//! - `include!`
//! - `include_str!`
//!
//! # Ignore Variable
//!
//! When any unknown variables are encountered, `ident_str!` will error, if that behaviour is not
//! desired, you can add `#<var> = None` to the declarations:
//!
//! ```
//! # // Weird stringify! magic is to make this example compile
//! ident_str::ident_str! {
//!     #ignore = None
//! #   => const _: &str = stringify!(
//!     => #ignore
//! #   );
//! }
//! ```
//!
//! This expands into
//!
//! ```ignore
//! #ignore
//! ```

use std::collections::HashMap;

use macro_string::MacroString;
use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

enum Value {
    MacroString(MacroString),
    None,
}

impl Value {
    pub fn to_string(&self) -> Option<String> {
        match self {
            Value::MacroString(MacroString(n)) => Some(n.clone()),
            Value::None => None,
        }
    }
}

impl Parse for Value {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input
            .step(|x| {
                if let Some((ident, cursor)) = x.ident() {
                    if ident == "None" {
                        Ok((ident, cursor))
                    } else {
                        Err(x.error("Expected string or None"))
                    }
                } else {
                    Err(x.error("Expected string or None"))
                }
            })
            .is_ok()
        {
            Ok(Value::None)
        } else {
            Ok(Value::MacroString(input.parse()?))
        }
    }
}

struct Decl {
    _hash: Token![#],
    ident: Ident,
    _eq_token: Token![=],
    value: Value,
}

impl Decl {
    fn name_to_tokens(&self) -> TokenStream2 {
        let mut tokens = TokenStream2::new();
        self._hash.to_tokens(&mut tokens);
        self.ident.to_tokens(&mut tokens);
        tokens
    }
}

impl Parse for Decl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let hash = input.parse()?;
        let ident = input.parse()?;
        let eq_token = input.parse()?;
        let value = input.parse()?;
        Ok(Self {
            _hash: hash,
            ident,
            _eq_token: eq_token,
            value,
        })
    }
}

struct Decls {
    decls: Vec<Decl>,
    body: TokenStream2,
}

impl Parse for Decls {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            decls: {
                let mut decls = Vec::new();
                loop {
                    decls.push(input.parse()?);
                    let lookahead = input.lookahead1();
                    if lookahead.peek(Token![,]) {
                        let _: Token![,] = input.parse()?;
                        if input.peek(Token![=>]) {
                            let _: Token![=>] = input.parse()?;
                            break;
                        }
                    } else if lookahead.peek(Token![=>]) {
                        let _: Token![=>] = input.parse()?;
                        break;
                    } else {
                        Err(lookahead.error())?
                    }
                }
                decls
            },
            // TODO: emit warning when stream does not use a variable (proc_macro::Diagnostic to be
            // stabilised)
            body: {
                if input.peek(syn::token::Brace) {
                    let g: Group = input.parse()?;
                    g.stream()
                } else {
                    input.parse()?
                }
            },
        })
    }
}

fn append_error(errors: &mut Option<syn::Error>, new: syn::Error) {
    if let Some(errors) = errors {
        errors.combine(new);
    } else {
        *errors = Some(new);
    }
}

fn translate_stream(
    stream: TokenStream2,
    map: &HashMap<String, Decl>,
    errors: &mut Option<syn::Error>,
) -> TokenStream2 {
    let mut out = TokenStream2::new();
    let mut iter = stream.into_iter().peekable();
    while let Some(tok) = iter.next() {
        match tok {
            TokenTree::Ident(_) => out.append(tok),
            TokenTree::Group(group) => {
                let mut group = Group::new(
                    group.delimiter(),
                    translate_stream(group.stream(), map, errors),
                );
                group.set_span(group.span());
                out.append(TokenTree::Group(group));
            }
            TokenTree::Punct(ref p) if p.as_char() == '#' => {
                if let Some(TokenTree::Ident(_)) = iter.peek() {
                    let Some(TokenTree::Ident(ident)) = iter.next() else {
                        unreachable!();
                    };
                    let strident = ident.to_string();
                    if let Some(decl) = map.get(&strident) {
                        let ident = if let Some(value) = &decl.value.to_string() {
                            Ident::new(value, ident.span()) // this won't panic, as we checked the string in main
                        } else {
                            out.append(tok);
                            ident
                        };
                        out.append(TokenTree::Ident(ident));
                    } else {
                        let mut tokens = TokenStream2::new();
                        tokens.append(tok);
                        tokens.append(ident);
                        append_error(
                            errors,
                            // TODO: Replace this with Diagnostic to get better error message
                            syn::Error::new_spanned(
                                tokens,
                                format!(
                                    "Unknown ident variable.  If you intended to literally use '#{0}', add `#{0} = None` to the declarations",
                                    strident
                                ),
                            ),
                        );
                        continue;
                    }
                } else {
                    out.append(tok);
                }
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => out.append(tok),
        }
    }

    out
}

/// The main macro.
///
/// Accepts any number of `name = <string literal>` pairs (or using macros like `concat!` or
/// `stringify!`) separated by commas (with an optional trailing comma), followed by `=>` and then
/// the body that will be expanded (optionally surrounded by `{}`).
///
/// ```
/// ident_str::ident_str! {
///     #name = "hello_world" =>
///     fn #name() -> &'static str {
///         stringify!(#name)
///     }
/// }
///
/// # fn main() {
/// #     assert_eq!(hello_world(), "hello_world");
/// # }
/// ```
#[proc_macro]
pub fn ident_str(input: TokenStream) -> TokenStream {
    let decls = parse_macro_input!(input as Decls);
    let mut map = HashMap::<String, Decl>::with_capacity(decls.decls.len());
    let mut errors: Option<syn::Error> = None;
    let mut can_continue = true;
    for d in decls.decls.into_iter() {
        let strident = d.ident.to_string();
        let existing = map.get(&strident);
        if existing.is_some() {
            append_error(
                &mut errors,
                syn::Error::new_spanned(
                    d.name_to_tokens(),
                    format!("Redefinition of #{}", d.ident),
                ),
            );
        }

        if let Some(valstring) = d.value.to_string() {
            if syn::parse_str::<Ident>(&valstring).is_err() {
                append_error(
                    &mut errors,
                    syn::Error::new_spanned(
                        d.name_to_tokens(),
                        format!("Invalid identifier: {:?}", valstring),
                    ),
                );
                can_continue = false;
            }
        }
        map.insert(strident, d);
    }

    let mut tokens = if can_continue {
        translate_stream(decls.body, &map, &mut errors)
    } else {
        debug_assert!(errors.is_some());
        TokenStream2::new()
    };

    if let Some(errors) = errors {
        errors.to_compile_error().to_tokens(&mut tokens);
    }

    tokens.into()
}
