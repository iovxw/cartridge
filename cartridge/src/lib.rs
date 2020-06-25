pub use cartridge_macros::{init_cartridges,cartridge};

#[doc(hidden)]
pub mod re_exports {
    pub use serde;
}

#[doc(hidden)]
pub mod internal {
    pub trait IsCartridge {
        const ID: u64;
    }

    pub fn call(method_id: u64) {}
    pub fn send<T>(v:T) {}
    pub fn recv<T>() -> T {todo!()}
}
