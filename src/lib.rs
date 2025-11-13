//! A macro to convert string literals into identifiers.  This is primarily
//! used for creating unique identifiers in macros.
//!
//! # Example
//!
//! ```
//! macro_rules! my_macro {
//!     ($name: ident) => {
//!         ident_str::ident_str! {
//!             #name_a = concat!(stringify!($name), "_a"),
//!             #name_b = concat!(stringify!($name), "_b")
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

use std::collections::HashMap;

use macro_string::MacroString;
use proc_macro::TokenStream;
use proc_macro2::{Group, Ident, TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

enum Value {
    String(LitStr),
    MacroString(MacroString),
    None(Ident),
}

impl Value {
    pub fn to_string(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.value()),
            Value::MacroString(MacroString(n)) => Some(n.clone()),
            Value::None(_) => None,
        }
    }
}

impl Parse for Value {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(ident) = input.step(|x| {
            if let Some((ident, cursor)) = x.ident()
                && ident == "None"
            {
                Ok((ident, cursor))
            } else {
                Err(x.error("Expected string or None"))
            }
        }) {
            Ok(Value::None(ident))
        } else if input.peek(LitStr) {
            Ok(Value::String(input.parse()?))
        } else {
            Ok(Value::MacroString(input.parse()?))
        }
    }
}

impl ToTokens for Value {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Value::String(lit_str) => lit_str.to_tokens(tokens),
            Value::MacroString(_) => {}
            Value::None(ident) => ident.to_tokens(tokens),
        }
    }
}

struct Decl {
    _hash: Token![#],
    ident: Ident,
    _eq_token: Token![=],
    value: Value,
}

impl ToTokens for Decl {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self._hash.to_tokens(tokens);
        self.ident.to_tokens(tokens);
        self._eq_token.to_tokens(tokens);
        self.value.to_tokens(tokens);
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
    let mut out = TokenStream2::new();
    let mut iter = stream.into_iter().peekable();
    while let Some(tok) = iter.next() {
        match tok {
            TokenTree::Ident(_) => out.append(tok),
            TokenTree::Group(group) => {
                let mut group =
                    Group::new(group.delimiter(), translate_stream(group.stream(), map)?);
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
                        if let Some(value) = &decl.value.to_string() {
                            let ident: Ident = match syn::parse_str::<Ident>(value) {
                                Ok(mut i) => {
                                    i.set_span(ident.span());
                                    i
                                }
                                Err(_) => {
                                    return Err(syn::Error::new(
                                        ident.span(),
                                        format!("Invalid identifier: {:?}", value),
                                    )
                                    .to_compile_error());
                                }
                            };
                            out.append(ident);
                        } else {
                            out.append(tok);
                            out.append(TokenTree::Ident(ident));
                        }
                    } else if !strident.starts_with("r#") {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("Unknown ident string.  If you intended to literally use '#{0}', you can:\n- Prefix it with 'r#': #r#{0}\n- Add `#{0} = None` to the declarations", strident)
                        )
                        .to_compile_error());
                    } else {
                        out.append(tok);
                        out.append(TokenTree::Ident(ident));
                    }
                } else {
                    out.append(tok);
                }
            }
            TokenTree::Punct(_) | TokenTree::Literal(_) => out.append(tok),
        }
    }

    Ok(out)
}

/// The main macro.
///
/// Accepts any number of `name = <string literal>` pairs (or using macros like `concat!` or
/// `stringify!`), followed by `=>` and then the body that will be expanded (optionally surrounded
/// by `{}`).
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
///
#[proc_macro]
pub fn ident_str(input: TokenStream) -> TokenStream {
    let decls = parse_macro_input!(input as Decls);
    let mut map = HashMap::<String, Decl>::with_capacity(decls.decls.len());
    for d in decls.decls.into_iter() {
        let strident = d.ident.to_string();
        let existing = map.get(&strident);
        if existing.is_some()
            && (!matches!(
                existing,
                Some(Decl {
                    value: Value::None(_),
                    ..
                })
            ) && matches!(d.value, Value::None(_)))
        {
            return syn::Error::new_spanned(&d, format!("Redefinition of #{}", d.ident))
                .to_compile_error()
                .into();
        } else {
            map.insert(strident, d);
        }
    }
    let tokens = translate_stream(decls.body, &map);

    match tokens {
        Ok(t) => t,
        Err(t) => t,
    }
    .into()
}
