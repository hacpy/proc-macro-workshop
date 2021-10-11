use proc_macro2::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    eprintln!("input:{:#?}", input);
    TokenStream::new().into()
}
