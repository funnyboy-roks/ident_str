struct MyStruct {
    name: &'static str,
}

ident_str::ident_str! {
    #name = "my_function" =>
    fn #name(s: MyStruct) -> &'static str {
        s.name
    }
}

#[test]
fn non_hash_ident() {
    assert_eq!(
        my_function(MyStruct {
            name: "hello world"
        }),
        "hello world"
    );
}

// This is obviously an exaggerated example, but its goal is to ensure that identifier translations
// work in every position.
ident_str::ident_str! {
    #derive = "derive",
    #derive = "derive",
    #debug = "Debug",
    #peq = "PartialEq",
    #eq = "Eq",
    #enumm = "MyEnum",
    #variant = "MyVariant",
    #variant2 = "MyVariant2",
    #generic = "T",
    #name = "add",
    #std = "std",
    #ops = "ops",
    #add = "Add",
    #output = "Output",
    #arg = "arg",
    #var = "n",
    #left = "left",
    #right = "right"
    =>

    #[#derive(#debug,#peq,#eq)]
    pub enum #enumm<#generic> {
        #variant,
        #variant2(#generic),
    }

    impl<#generic> #enumm<#generic> {
        fn #name(self, #arg: #enumm<#generic>) -> Self
            where #generic: #std::#ops::#add<#generic, #output = #generic>
        {
            use #enumm::*;
            match (self, #arg) {
                (#variant, #variant) => #variant,
                (#variant, #variant2(#var)) => #variant2(#var),
                (#variant2(#var), #variant) => #variant2(#var),
                (#variant2(#left), #variant2(#right)) => #variant2(#left + #right),
            }
        }
    }
}

#[test]
fn all_the_positions() {
    MyEnum::<u8>::MyVariant.add(MyEnum::MyVariant);
    assert_eq!(
        MyEnum::MyVariant2(2).add(MyEnum::MyVariant),
        MyEnum::MyVariant2(2)
    );
    assert_eq!(
        MyEnum::MyVariant.add(MyEnum::MyVariant2(2)),
        MyEnum::MyVariant2(2)
    );
    assert_eq!(
        MyEnum::MyVariant2(2).add(MyEnum::MyVariant2(2)),
        MyEnum::MyVariant2(4)
    );
}
