extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, PathArguments};

fn is_option(ty: &syn::Type) -> Option<&syn::PathSegment> {
    let segments = if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        segments
    } else {
        unreachable!();
    };
    segments.iter().find(|it| it.ident == "Option")
}

#[proc_macro_derive(Builder)]
#[allow(unused)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    let builder_ident = format_ident!("{}Builder", derive_input.ident);
    let ident = derive_input.ident;
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = derive_input.data
    {
        named
    } else {
        unreachable!();
    };

    let fields_iter = fields.iter().map(|field| {
        let name = field.ident.clone().unwrap();
        let ty = &field.ty;
        if is_option(ty).is_some() {
            quote! {
                #name: #ty
            }
        } else {
            quote! {
                #name: std::option::Option<#ty>
            }
        }
    });

    let build_field_args = fields.iter().map(|field| {
        let name = field.ident.clone().unwrap();
        quote! {
            #name: None
        }
    });

    let build_command_args = fields.iter().map(|field| {
        let name = field.ident.clone().unwrap();
        let fmt = format!("{} parameter is not set", name);
        if is_option(&field.ty).is_some() {
            quote! {
                #name: self.#name.clone()
            }
        } else {
            quote! {
                #name: self.#name.clone().ok_or(#fmt)?
            }
        }
    });

    let build_setter = fields.iter().map(|field| {
        let arg_name = format_ident!("{}", field.ident.clone().unwrap());
        let mut ty = field.ty.clone();
        let generic_args = if let Some(seg) = is_option(&ty) {
            match &seg.arguments {
                PathArguments::AngleBracketed(ref ag) => {
                    let args = ag.clone().args;
                    quote! {
                        #args
                    }
                }
                _ => unimplemented!(),
            }
        } else {
            quote! {
                #ty
            }
        };

        quote! {
            pub fn #arg_name(&mut self, #arg_name: #generic_args) -> &mut Self{
                self.#arg_name = Some(#arg_name);
                self
            }
        }
    });

    let builder_ident_token = quote! {
        pub struct #builder_ident{
            #(#fields_iter), *
        }

        impl #builder_ident{
            #(#build_setter)*

            pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>>{
                Ok(#ident{
                    #(#build_command_args), *
                })
            }
        }

        impl #ident{
            pub fn builder() -> #builder_ident{
                #builder_ident{
                    #(#build_field_args),*
                }
            }
        }
    };

    TokenStream::from(builder_ident_token)
}
