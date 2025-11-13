<!-- Readme generated with `cargo-readme`: https://github.com/webern/cargo-readme -->

# ident-str

<!-- [![Crates.io](https://img.shields.io/crates/v/leucite.svg)](https://crates.io/crates/leucite) -->
<!-- [![Documentation](https://docs.rs/leucite/badge.svg)](https://docs.rs/leucite/) -->
<!-- [![Dependency status](https://deps.rs/repo/github/basalt-rs/leucite/status.svg)](https://deps.rs/repo/github/basalt-rs/leucite) -->

A macro to convert string literals into identifiers.  The primary use-case is to allow
declarative macros to produce identifiers that are unique to each call.  The alternative is to
accept each identifier as an argument to the macro call, which gets unweildy with many
identifiers.

## Usage

```rust
macro_rules! my_macro {
    ($name: ident) => {
        ident_str::ident_str! {
            #name_a = concat!(stringify!($name), "_a"),
            #name_b = concat!(stringify!($name), "_b")
            => {
                fn #name_a() {}
                fn #name_b() {}
            }
        }
    };
}

my_macro!(foo);
```

expands to

```rust
fn foo_a() {}
fn foo_b() {}
```

## Supported Macros

This crate takes advantage of [`MacroString`](https://github.com/dtolnay/macro-string/) and
supports all macros supported by it.  Those macros are:
- `concat!`
- `stringify!`
- `env!`
- `include!`
- `include_str!`

## Ignore Variable

When any unknown variables are encountered, `ident_str!` will error, if that behaviour is not
desired, you can add `#<var> = None` to the declarations:

```rust
ident_str::ident_str! {
    #ignore = None
    => #ignore
}
```

This exapands into

```rust
#ignore
```
