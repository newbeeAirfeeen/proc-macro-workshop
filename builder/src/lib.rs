extern crate proc_macro;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Lit, PathArguments};

#[allow(unused)]
fn is_type<'a>(ty: &'a syn::Type, type_str: &'a str) -> Option<&'a syn::PathSegment> {
    let segments = if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        segments
    } else {
        unreachable!();
    };

    let t = type_str.to_string();
    segments.iter().find(|it| it.ident == t)
}

fn get_attribute_token_stream(field: &syn::Field) -> std::option::Option<&syn::Attribute> {
    if field.attrs.len() == 1 {
        return Some(field.attrs.first().unwrap());
    }
    None
}

fn get_inner_args(field: &syn::Field, outer_arg_type_name: &str) -> proc_macro2::TokenStream {
    if let Some(seg) = is_type(&field.ty, outer_arg_type_name) {
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
        let ty = field.ty.clone();
        quote! {
            #ty
        }
    }
}

fn builder_each_arg(field: &syn::Field) -> (proc_macro2::TokenStream, bool) {
    let mut builder_each = quote! { /*.......*/};
    let mut build = false;
    if let Some(attr) = get_attribute_token_stream(field) {
        let token = attr.clone().tokens;
        let token_tree = token.into_iter().next().unwrap();
        if let proc_macro2::TokenTree::Group(ref group) = token_tree {
            let mut stream = group.clone().stream().into_iter();
            let token_tree = stream.next().unwrap();
            if let proc_macro2::TokenTree::Ident(id) = token_tree {
                if id != "each" {
                    unimplemented!();
                }
            }
            let token_tree = stream.next().unwrap();
            if let proc_macro2::TokenTree::Punct(punct) = token_tree {
                if punct.as_char() != '=' || punct.spacing() != proc_macro2::Spacing::Alone {
                    unimplemented!();
                }
            }

            let token_tree = stream.next().unwrap();
            if let proc_macro2::TokenTree::Literal(literal) = token_tree {
                let lit = syn::Lit::new(literal);
                let lit_str = match lit {
                    Lit::Str(s) => s,
                    _ => {
                        unimplemented!();
                    }
                };
                let lit_str = format_ident!("{}", lit_str.value());
                let inner_type = get_inner_args(field, "Vec");
                let arg_name = format_ident!("{}", field.ident.clone().unwrap());
                builder_each = quote! {
                    pub fn #lit_str(&mut self, #lit_str: #inner_type) -> &mut Self{
                        if self.#arg_name.is_none() {
                            self.#arg_name = Some(Vec::<#inner_type>::new());
                        }
                        match self.#arg_name.as_mut() {
                            Some(inner_arg) => {
                                inner_arg.push(#lit_str);
                            },
                            None => unimplemented!()
                        };
                        self
                    }
                };
                build = true;
            }
        }
    }
    (builder_each, build)
}

fn build_macro(derive_input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
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
        if is_type(ty, "Option").is_some() {
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
        if is_type(&field.ty, "Option").is_some() {
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
        let (builder_each, is_build) = builder_each_arg(field);
        let generic_args = get_inner_args(field, "Option");
        let mut to = quote! {
            pub fn #arg_name(&mut self, #arg_name: #generic_args) -> &mut Self{
                self.#arg_name = Some(#arg_name);
                self
            }
        };
        if is_build {
            to = quote! {};
        }
        quote! {
            #builder_each
            #to
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
    Ok(builder_ident_token)
}

#[proc_macro_derive(Builder, attributes(builder))]
#[allow(unused)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match build_macro(input) {
        Ok(stream) => stream.into(),
        Err(err) => {
            // let mut path = std::env::current_exe().unwrap();
            // path.push(".stderr.show");
            // std::fs::File::create(path).unwrap().write(err.to_string().as_bytes());
            // proc_macro::TokenStream::from(quote! {})
            //err.to_compile_error();
            proc_macro::TokenStream::from(quote!{})
        }

    }
}
