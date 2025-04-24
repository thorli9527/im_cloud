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
            let field_ident = match field.ident.clone() {
                Some(ident) => ident,
                None => continue, // 跳过无名字段
            };

            // 默认跳过字段，只有含 #[query] 才处理
            let mut should_generate = false;
            let mut query_type = String::new();
            let mut rename = field_ident.to_string();

            for attr in field.attrs.iter().filter(|a| a.path().is_ident("query")) {
                should_generate = true;

                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("eq") {
                        query_type = "eq".into();
                    } else if meta.path.is_ident("between") {
                        query_type = "between".into();
                    } else if meta.path.is_ident("gt") {
                        query_type = "gt".into();
                    } else if meta.path.is_ident("lt") {
                        query_type = "lt".into();
                    } else if meta.path.is_ident("like") {
                        query_type = "like".into();
                    } else if meta.path.is_ident("field") {
                        if let Ok(val) = meta.value()?.parse::<syn::LitStr>() {
                            rename = val.value();
                        }
                    }
                    Ok(())
                });
            }

            if !should_generate {
                continue;
            }

            let handler = match query_type.as_str() {
                "eq" => quote! {
                    if let Some(val) = &self.#field_ident {
                        doc.insert(#rename, bson::to_bson(val).unwrap());
                    }
                },
                "gt" => quote! {
                    if let Some(val) = &self.#field_ident {
                        doc.insert(#rename, doc! { "$gt": val });
                    }
                },
                "lt" => quote! {
                    if let Some(val) = &self.#field_ident {
                        doc.insert(#rename, doc! { "$lt": val });
                    }
                },
                "between" => quote! {
                    if let Some(val) = &self.#field_ident {
                        doc.insert(#rename, doc! { "$gte": val, "$lte": val });
                    }
                },
                "like" => quote! {
                    if let Some(val) = &self.#field_ident {
                        doc.insert(#rename, doc! { "$regex": val, "$options": "i" });
                    }
                },
                _ => quote!(),
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
