use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Data, DeriveInput, Error, Fields, Lit, Meta, MetaList,
    NestedMeta, Result, Type,
};

#[proc_macro_derive(Builder, attributes(builder))]
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
    let builder_struct = match create_builder_struct(&fields, &builder_name) {
        Ok(t) => t,
        Err(e) => return e.to_compile_error().into(),
    };

    // impl #name {
    //      pub fn builder() -> Builder {}
    //}
    let builder = impl_builder(&fields, &builder_name).unwrap_or_else(|err| err.to_compile_error());
    // impl Builder {
    //      pub fn build() -> #name {}
    //}
    let builder_build =
        impl_builder_build(&fields, &name).unwrap_or_else(|err| err.to_compile_error());
    // impl Builder {
    //      fn set_func() -> &mut Self {}
    // }
    let builder_funcs = impl_builder_funcs(&fields).unwrap_or_else(|err| err.to_compile_error());

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

fn get_attr_value(field: &syn::Field) -> Result<Option<Ident>> {
    for attr in &field.attrs {
        if let Ok(Meta::List(MetaList {
            ref path,
            ref nested,
            ..
        })) = attr.parse_meta()
        {
            if let Some(p) = path.segments.first() {
                if p.ident == "builder" {
                    if let Some(NestedMeta::Meta(Meta::NameValue(kv))) = nested.first() {
                        if kv.path.is_ident("each") {
                            if let Lit::Str(ref ident_str) = kv.lit {
                                return Ok(Some(Ident::new(
                                    ident_str.value().as_str(),
                                    attr.span(),
                                )));
                            }
                        } else {
                            if let Ok(syn::Meta::List(ref list)) = attr.parse_meta() {
                                return Err(Error::new_spanned(
                                    list,
                                    r#"expected `builder(each = "...")`"#,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn get_inner_type<'a>(ty: &'a Type, ident_name: &str) -> Option<&'a Type> {
    if let syn::Type::Path(syn::TypePath { ref path, .. }) = ty {
        if let Some(seg) = path.segments.last() {
            if seg.ident == ident_name {
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    ref args,
                    ..
                }) = seg.arguments
                {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

fn is_optional_field(ty: &Type) -> bool {
    get_inner_type(ty, "Option").is_some()
}

fn create_builder_struct(fields: &Fields, builder_name: &Ident) -> Result<TokenStream> {
    let new_fields: Result<Vec<TokenStream>> = fields
        .iter()
        .map(|field| {
            let name = &field.ident.to_owned().unwrap();
            let ty = &field.ty;

            if get_attr_value(field)?.is_some() {
                Ok(quote! {
                    #name: #ty
                })
            } else if is_optional_field(ty) {
                Ok(quote! {
                    #name: #ty
                })
            } else {
                Ok(quote! {
                    #name: std::option::Option<#ty>
                })
            }
        })
        .collect();

    let new_fields = new_fields?;
    Ok(quote! {
        pub struct #builder_name {
            #( #new_fields ),*
        }
    })
}

fn impl_builder(fields: &Fields, builder_name: &Ident) -> Result<TokenStream> {
    let new_fields: Result<Vec<TokenStream>> = fields
        .iter()
        .map(|field| {
            let name = &field.ident.to_owned().unwrap();
            if get_attr_value(field)?.is_some() {
                Ok(quote! {
                    #name: std::vec::Vec::new(),
                })
            } else {
                Ok(quote! {
                    #name: None,
                })
            }
        })
        .collect();
    let new_fields = new_fields?;
    Ok(quote! {
        pub fn builder() -> #builder_name {
            #builder_name {
                #(#new_fields)*
            }
        }
    })
}

fn impl_builder_build(fields: &Fields, name: &Ident) -> Result<TokenStream> {
    let new_fields: Result<Vec<TokenStream>> = fields
        .iter()
        .map(|field| {
            let name = &field.ident.to_owned().unwrap();
            let ty = &field.ty;
            if get_attr_value(field)?.is_some() {
                Ok(quote! {
                    #name: self.#name.clone(),
                })
            } else if is_optional_field(ty) {
                Ok(quote! {
                    #name: self.#name.take(),
                })
            } else {
                Ok(quote! {
                    #name: self.#name.take().ok_or_else(|| "???")?,
                })
            }
        })
        .collect();

    let new_fields = new_fields?;

    Ok(quote! {
        pub fn build(&mut self) -> std::result::Result<#name, std::boxed::Box<dyn std::error::Error>> {
            Ok(#name {
                #(#new_fields)*
            })
        }
    })
}

fn impl_builder_funcs(fields: &Fields) -> Result<TokenStream> {
    let set_calls: Result<Vec<TokenStream>> = fields
        .iter()
        .map(|field| {
            let name = &field.ident.to_owned().unwrap();
            let ty = &field.ty;
            if get_attr_value(field)?.is_some() {
                let value = get_attr_value(field)?.unwrap();
                let inner_ty = get_inner_type(ty, "Vec").unwrap();
                let mut q = quote! {
                    fn #value(&mut self, #value: #inner_ty) -> &mut Self {
                        self.#name.push(#value);
                        self
                    }
                };
                if name != &value {
                    q.extend(quote! {
                        fn #name(&mut self, #name: #ty) -> &mut Self {
                            self.#name = #name;
                            self
                        }
                    })
                }
                Ok(q)
            } else if is_optional_field(ty) {
                Ok(quote! {
                    fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name = #name;
                        self
                    }
                })
            } else {
                Ok(quote! {
                    fn #name(&mut self, #name: #ty) -> &mut Self {
                        self.#name = Some(#name);
                        self
                    }
                })
            }
        })
        .collect();
    let set_calls = set_calls?;
    Ok(quote! {
        #( #set_calls )*
    })
}
