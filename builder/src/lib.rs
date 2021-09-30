use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let builder_name = Ident::new(&format!("{}Builder", name), Span::call_site());

    let data = input.data;
    let fields = match &data {
        Data::Struct(data) => data.fields.to_owned(),
        Data::Enum(_) => todo!(),
        Data::Union(_) => todo!(),
    };

    // pub struct Builder {}
    let builder_struct = create_builder_struct(&fields, &builder_name);
    // impl #name {
    //      pub fn builder() -> Builder {}
    //}
    let builder = impl_builder(&fields, &builder_name);
    // impl Builder {
    //      pub fn build() -> #name {}
    //}
    let builder_build = impl_builder_build(&fields, &name);
    // impl Builder {
    //      fn set_func() -> &mut Self {}
    // }
    let builder_funcs = impl_builder_funcs(&fields);

    let expand = quote! {
        #builder_struct

        impl #name {
            #builder
        }

        impl #builder_name {
            #builder_build

            #builder_funcs
        }
    };

    expand.into()
}

fn is_optional_field(ty: &Type) -> bool {
    let seg = match ty {
        Type::Path(p) => p.path.segments.first().unwrap(),
        _ => todo!(),
    };
    seg.ident.to_string() == "Option"
}

fn create_builder_struct(fields: &Fields, builder_name: &Ident) -> TokenStream {
    let new_fields = fields.iter().map(|field| {
        let name = &field.ident.to_owned().unwrap();
        let ty = &field.ty;
        if is_optional_field(ty) {
            quote! {
                #name: #ty
            }
        } else {
            quote! {
                #name: Option<#ty>
            }
        }
    });

    quote! {
        pub struct #builder_name {
            #( #new_fields ),*
        }
    }
}

fn impl_builder(fields: &Fields, builder_name: &Ident) -> TokenStream {
    let new_fields = fields.iter().map(|field| {
        let name = &field.ident.to_owned().unwrap();
        quote! {
            #name: None,
        }
    });

    quote! {
        pub fn builder() -> #builder_name {
            #builder_name {
                #(#new_fields)*
            }
        }
    }
}

fn impl_builder_build(fields: &Fields, name: &Ident) -> TokenStream {
    let new_fields = fields.iter().map(|field| {
        let name = &field.ident.to_owned().unwrap();
        let ty = &field.ty;
        if is_optional_field(ty) {
            quote! {
                #name: self.#name.take(),
            }
        } else {
            quote! {
                #name: self.#name.take().ok_or_else(|| "???")?,
            }
        }
    });

    quote! {
        pub fn build(&mut self) -> std::result::Result<#name, std::boxed::Box<dyn std::error::Error>> {
            Ok(#name {
                #(#new_fields)*
            })
        }
    }
}

fn impl_builder_funcs(fields: &Fields) -> TokenStream {
    let set_calls = fields.iter().map(|field| {
        let name = &field.ident.to_owned().unwrap();
        let ty = &field.ty;
        if is_optional_field(ty) {
            quote! {
                fn #name(&mut self, #name: #ty) -> &mut Self {
                    self.#name = #name;
                    self
                }
            }
        } else {
            quote! {
                fn #name(&mut self, #name: #ty) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
            }
        }
    });

    quote! {
        #( #set_calls )*
    }
}
