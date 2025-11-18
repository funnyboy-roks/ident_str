// cargo test --example test_cases

use std::ops::Add;

pub fn add<T, U, V>(left: T, right: U) -> V
where
    T: Add<U, Output = V>,
{
    left + right
}

macro_rules! test_add {
    ($left: literal + $right: literal == $result: literal) => {
        ident_str::ident_str! {
            #name = concat!("test_add_", $left, "_", $right, "_eq_", $result),
            => {
                #[test]
                fn #name() {
                    assert_eq!(add($left, $right), $result);
                }
            }
        }
    };
}

test_add!(1 + 2 == 3);
test_add!(2 + 2 == 4);
test_add!(3 + 2 == 5);
test_add!(4 + 2 == 6);
test_add!(2 + 3 == 5);
test_add!(3 + 4 == 7);
test_add!(4 + 5 == 9);
