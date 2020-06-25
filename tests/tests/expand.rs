use macrotest;

#[test]
#[ignore]
pub fn basic() {
    macrotest::expand("tests/expand/basic.rs");
}
