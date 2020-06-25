extern crate proc_macro;
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use lazy_static::lazy_static;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    Block, DeriveInput, Expr, FnArg, Generics, Ident, ImplItem, ImplItemMethod, ItemImpl, Pat,
    PatIdent, PatType, Path, PathSegment, Result, ReturnType, Stmt, Token, Type, TypePath,
    Visibility,
};
use proc_macro_crate::crate_name;

lazy_static! {
    static ref CARTRIDGE_COUNTER: AtomicU64 = AtomicU64::default();
}

#[proc_macro]
pub fn init_cartridges(args: TokenStream) -> TokenStream {
    let InitArgs(cartridge_types) = parse_macro_input!(args as InitArgs);
    let cartridge_crate = find_cartridge_crate();
    let out = quote!({
        match 0 {
            #( <#cartridge_types as ::#cartridge_crate::internal::IsCartridge>::ID => {} )*
            _ => unreachable!(),
        }
    });
    out.into()
}

struct InitArgs(Vec<Type>);

impl Parse for InitArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let args: Punctuated<Type, Token![,]> = input.parse_terminated(Parse::parse)?;
        Ok(InitArgs(args.into_iter().collect()))
    }
}

#[proc_macro_attribute]
pub fn cartridge(attr: TokenStream, item: TokenStream) -> TokenStream {
    let struct_ = parse_macro_input!(attr as CartridgeStruct);
    let mut original = item.clone();
    let impls = parse_macro_input!(item as CartridgeImpl);

    let name = struct_.name;
    let pub_ = struct_.visibility;
    let inner_type = impls.inner_type;
    let (impl_generics, ty_generics, where_clause) = impls.generics.split_for_impl();
    let mut items = impls.items;
    let id = CARTRIDGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    for (method_id, item) in items.iter_mut().enumerate() {
        match item {
            // constructor
            ImplItem::Method(method) if method_is_constructor(method) => {
                // TODO: replace body
                method.block.stmts = vec![parse_quote! { todo!(); }];
            }
            // normal static method
            ImplItem::Method(method) if method_is_static(method) => {
                // ignore
            }
            // normal method
            ImplItem::Method(method) => {
                // TODO: replace body
                let assert_ser_and_deser = gen_serde_asserts(&method);
                let body = gen_method_body(&method, method_id as u64);
                method.block.stmts = vec![assert_ser_and_deser, body];
            }
            // others
            _ => {}
        }
    }
    let cartridge_crate = find_cartridge_crate();
    let out = quote! {
        #pub_ struct #name #impl_generics #where_clause {
            inner: #inner_type,
        }

        impl #impl_generics #name #ty_generics #where_clause {
            #( #items )*
        }

        impl ::#cartridge_crate::internal::IsCartridge for #name {
            const ID: u64 = #id;
        }
    };

    original.extend(TokenStream::from(out));
    original
}

fn gen_serde_asserts(method: &ImplItemMethod) -> Stmt {
    let mut method_types: Vec<&Type> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(arg) = arg {
                Some(&*arg.ty)
            } else {
                None
            }
        })
        .collect();
    if let ReturnType::Type(_, output) = &method.sig.output {
        method_types.push(output)
    }
    let cartridge_crate = find_cartridge_crate();

    parse_quote! {{
        fn __assert_serialize_deserialize<'de, T>()
        where
            T: ::#cartridge_crate::re_exports::serde::Serialize
            + ::#cartridge_crate::re_exports::serde::Deserialize<'de>
        {
        }

        #( __assert_serialize_deserialize::<#method_types>(); )*
    }}
}

fn gen_method_body(method: &ImplItemMethod, method_id: u64) -> Stmt {
    let args: Vec<&Pat> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(arg) = arg {
                Some(&*arg.pat)
            } else {
                None
            }
        })
        .collect();
    let cartridge_crate = find_cartridge_crate();
    parse_quote! {{
        #cartridge_crate::internal::call(#method_id);
        #( #cartridge_crate::internal::send(,#args); )*
        #cartridge_crate::internal::recv()
    }}
}

struct CartridgeImpl {
    generics: Generics,
    inner_type: Box<Type>,
    items: Vec<ImplItem>,
}

impl Parse for CartridgeImpl {
    fn parse(input: ParseStream) -> Result<Self> {
        let impl_block: ItemImpl = input.parse()?;
        let mut errors = Vec::new();

        if let Some((_, path, _)) = impl_block.trait_ {
            //path.span().unwrap().error("TODO: fix this message").emit();
            let error = syn::Error::new(path.span(), "TODO: fix this message");
            errors.push(error);
        }
        if !impl_block.generics.params.is_empty() {
            let error = syn::Error::new(impl_block.generics.span(), "TODO");
            errors.push(error);
        }

        let mut found_constructor = false;
        for item in &impl_block.items {
            if let ImplItem::Method(method) = item {
                if let Some(token) = method.sig.asyncness {
                    let error =
                        syn::Error::new(token.span(), "async function in cartridge is not allowed");
                    errors.push(error);
                    //token
                    //    .span()
                    //    .unwrap()
                    //    .error("async function in cartridge is not allowed")
                    //    .help("try #[async_cartridge]")
                    //    .emit();
                }
                if !method.sig.generics.params.is_empty() {
                    let error = syn::Error::new(method.sig.generics.span(), "TODO");
                    errors.push(error);
                }

                // return type must be Self
                // impl X { fn new() -> X } is not allowed
                // must be: impl X { fn new() -> Self }
                match method.sig.receiver() {
                    Some(FnArg::Typed(_)) => {
                        let span = method.sig.span();
                        let error = syn::Error::new(span, "TODO: normal static method");
                        errors.push(error);
                    }
                    None if method_is_constructor(&method) => {
                        found_constructor = true;
                    }
                    _ => (),
                }
            }
        }
        if !found_constructor {
            // TODO: move the error note to a real note when `proc_macro_diagnostic` is stabled
            errors.push(input.error(
                "constructor not found. \
                 the constructor must be a static method that returns `Self`",
            )); // why must `Self`? because we don't modify the signature
        }
        if let Some(mut error) = errors.pop() {
            for another_error in errors {
                error.combine(another_error);
            }
            return Err(error);
        }
        Ok(Self {
            generics: impl_block.generics,
            inner_type: impl_block.self_ty,
            items: impl_block.items,
        })
    }
}

fn method_is_static(method: &ImplItemMethod) -> bool {
    method.sig.receiver().is_none()
}

fn method_is_constructor(method: &ImplItemMethod) -> bool {
    let output_is_self = matches!(
        &method.sig.output,
        ReturnType::Type(_, type_)
            if matches!(&**type_, Type::Path(TypePath {path: Path {segments, ..},..})
                        if matches!(segments.first(), Some(PathSegment {ident,..})
                                    if ident == "Self"
                        )
            )
    );

    output_is_self && method_is_static(method)
}

struct CartridgeStruct {
    visibility: Visibility,
    name: Ident,
}

impl Parse for CartridgeStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.is_empty() {
            return Err(input.error("please name this cartridge"));
        }
        let visibility = input.parse()?;
        let name = input.parse()?;
        Ok(Self { visibility, name })
    }
}

fn find_cartridge_crate() -> Ident {
    let name = crate_name("cartridge").unwrap();
    Ident::new(&name, Span::call_site())
}
