//! A macro to convert string literals into identifiers.  This is primarily
//! used for creating unique identifiers in macros.
//!
//! # Example
//!
//! ```
//! macro_rules! my_macro {
//!     ($name: ident) => {
//!         ident_str::ident_str! {
//!             name_a = concat!(stringify!($name), "_a"),
//!             name_b = concat!(stringify!($name), "_b")
//!             => {
//!                 fn name_a() {}
//!                 fn name_b() {}
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

use std::collections::HashMap;

use macro_string::MacroString;
use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, TokenStream as TokenStream2, TokenTree};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Token,
};

struct Decl {
    ident: Ident,
    _eq_token: Token![=],
    value: MacroString,
}

impl Parse for Decl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        let eq_token = input.parse()?;
        let value = input.parse()?;
        Ok(Self {
            ident,
            _eq_token: eq_token,
            value,
        })
    }
}

struct Decls {
    decls: Punctuated<Decl, Token![,]>,
    _arrow: Token![=>],
    body: TokenStream2,
}

impl Parse for Decls {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            decls: Punctuated::parse_separated_nonempty(input)?,
            _arrow: input.parse()?,
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

fn translate_stream(
    stream: TokenStream2,
    map: &HashMap<String, Decl>,
) -> Result<TokenStream2, TokenStream2> {
    stream
        .into_iter()
        .map(|tok| match tok {
            TokenTree::Ident(ident) => {
                if let Some(decl) = map.get(&ident.to_string()) {
                    let ident: Ident = match syn::parse_str::<Ident>(&decl.value.0) {
                        Ok(mut ident) => {
                            ident.set_span(decl.ident.span());
                            ident
                        }
                        Err(_) => {
                            return Err(syn::Error::new(
                                ident.span(),
                                format!("Invalid identifier: {:?}", decl.value.0),
                            )
                            .to_compile_error());
                        }
                    };
                    Ok(TokenTree::Ident(ident))
                } else {
                    Ok(TokenTree::Ident(ident))
                }
            }
            TokenTree::Group(group) => {
                let mut group =
                    Group::new(group.delimiter(), translate_stream(group.stream(), map)?);
                group.set_span(group.span());
                Ok(TokenTree::Group(group))
            }
            TokenTree::Punct(_) => Ok(tok),
            TokenTree::Literal(_) => Ok(tok),
        })
        .collect()
}

/// The main macro.
///
/// Accepts any number of `name = <string literal>` pairs (or using macros like `concat!` or
/// `stringify!`), followed by `=>` and then the body that will be expanded (optionally surrounded
/// by `{}`).
///
/// ```
/// ident_str::ident_str! {
///     name = "hello_world" =>
///     fn name() -> &'static str {
///         stringify!(name)
///     }
/// }
///
/// # fn main() {
/// #     assert_eq!(hello_world(), "hello_world");
/// # }
/// ```
///
/// If you wish to use an identifier that you assigned in the assignments, you can use a raw
/// identifier: `r#<name>`:
///
/// ```
/// # struct MyStruct { name: &'static str }
/// ident_str::ident_str! {
///     name = "my_function" =>
///     fn name(s: MyStruct) -> &'static str {
///         s.r#name
///     }
/// }
///
/// # fn main() {
/// #     assert_eq!(my_function(MyStruct { name: "hello world" }), "hello world");
/// # }
/// ```
#[proc_macro]
pub fn ident_str(input: TokenStream) -> TokenStream {
    let decls = parse_macro_input!(input as Decls);
    let map = decls
        .decls
        .into_iter()
        .map(|d| (d.ident.to_string(), d))
        .collect::<HashMap<_, _>>();
    let tokens = translate_stream(decls.body, &map);

    match tokens {
        Ok(t) => t,
        Err(t) => t,
    }
    .into()
}
