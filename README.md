# ident_str

A macro to convert string literals into identifiers.  This is primarily
used for creating unique identifiers in macros.

## Example

(from [`examples/simple.rs`](./examples/simple.rs))

```rust
macro_rules! test_add {
    ($left: literal + $right: literal == $result: literal) => {
        ident_str::ident_str! {
            name = concat!("test_add_", $left, "_", $right, "_eq_", $result)
            => {
                #[test]
                fn name() {
                    assert_eq!(add($left, $right), $result);
                }
            }
        }
    };
}
```
