use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(CustomDebug)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;
    let data = input.data;
    let fields = match &data {
        Data::Struct(data) => data.fields.to_owned(),
        Data::Enum(_) => todo!(),
        Data::Union(_) => todo!(),
    };

    let new_fields = fields.iter().map(|field| {
        let name = &field.ident.to_owned().unwrap();
        let name_str = name.to_string();
        quote! {
            .field(#name_str, &self.#name)
        }
    });

    let name_str = name.to_string();
    let expand = quote! {
        impl std::fmt::Debug for #name {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                fmt.debug_struct(#name_str)
                    #( #new_fields )*
                    .finish()
            }
        }
    };

    expand.into()
}
