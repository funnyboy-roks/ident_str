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
