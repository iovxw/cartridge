extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, DeriveInput, Result,
};

#[proc_macro_attribute]
pub fn cartridge(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("attr: \"{}\"", attr.to_string());
    println!("item: \"{}\"", item.to_string());
    let input = parse_macro_input!(item as Cartridge);

    quote!().into()
}

struct Cartridge {}

impl Parse for Cartridge {
    fn parse(input: ParseStream) -> Result<Self> {
        todo!()
    }
}
