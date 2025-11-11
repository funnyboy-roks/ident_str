<!-- Readme generated with `cargo-readme`: https://github.com/webern/cargo-readme -->

# ident-str

<!-- [![Crates.io](https://img.shields.io/crates/v/leucite.svg)](https://crates.io/crates/leucite) -->
<!-- [![Documentation](https://docs.rs/leucite/badge.svg)](https://docs.rs/leucite/) -->
<!-- [![Dependency status](https://deps.rs/repo/github/basalt-rs/leucite/status.svg)](https://deps.rs/repo/github/basalt-rs/leucite) -->

A macro to convert string literals into identifiers.  This is primarily
used for creating unique identifiers in macros.

## Example

```rust
macro_rules! my_macro {
    ($name: ident) => {
        ident_str::ident_str! {
            name_a = concat!(stringify!($name), "_a"),
            name_b = concat!(stringify!($name), "_b")
            => {
                fn name_a() {}
                fn name_b() {}
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
