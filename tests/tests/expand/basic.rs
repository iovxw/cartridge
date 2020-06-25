use cartridge::cartridge;

struct MyStruct;

#[cartridge(MyCartridge)]
impl MyStruct {
    fn new() -> Self {
        MyStruct
    }

    fn method(&self) {}
}
