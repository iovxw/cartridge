#![feature(proc_macro_hygiene)]

use cartridge::{init_cartridges,cartridge};

struct MyStruct;

#[cartridge(MyCartridge)]
impl MyStruct {
    fn new() -> Self {
        MyStruct
    }

    fn method(&self) {}
}

#[cartridge(MyCartridge2)]
impl MyStruct {
    fn new2() -> Self {
        MyStruct
    }

    fn method2(&self) {}
}

#[test]
fn basic() {
    init_cartridges!(MyCartridge, MyCartridge2);
}
