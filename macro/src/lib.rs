use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};
#[proc_macro_derive(QueryFilter, attributes(query))]
pub fn derive_query_filter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let mut field_handlers = vec![];

    if let syn::Data::Struct(data) = input.data {
        for field in data.fields {
            let field_ident = field.ident.clone().unwrap();
            let mut query_type = "eq".to_string();
            let mut rename = field_ident.to_string();

            for attr in field.attrs.iter().filter(|a| a.path().is_ident("query")) {
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("eq") {
                        query_type = "eq".into();
                    } else if meta.path.is_ident("between") {
                        query_type = "between".into();
                    } else if meta.path.is_ident("field") {
                        if let Ok(val) = meta.value()?.parse::<syn::LitStr>() {
                            rename = val.value();
                        }
                    }
                    Ok(())
                });
            }

            let handler = if query_type == "eq" {
                quote! {
                    if let Some(val) = &self.#field_ident {
                        doc.insert(#rename, bson::to_bson(val).unwrap());
                    }
                }
            } else if query_type == "between" {
                quote! {
                    if let Some(val) = &self.#field_ident {
                        doc.insert(#rename, doc! { "$gte": val, "$lte": val });
                    }
                }
            } else {
                quote!()
            };

            field_handlers.push(handler);
        }
    }

    let output = quote! {
        impl #struct_name {
            pub fn to_query_doc(&self) -> mongodb::bson::Document {
                use mongodb::bson::{doc, Bson};
                let mut doc = doc! {};
                #(#field_handlers)*
                doc
            }
        }
    };

    output.into()
}
