use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_query_filter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;
    let mut field_handlers = vec![];

    if let syn::Data::Struct(data) = input.data {
        for field in data.fields {
            let field_ident = match &field.ident {
                Some(ident) => ident,
                None => continue,
            };

            let mut rename = field_ident.to_string();
            let mut query_ops = Vec::new();

            // 解析所有 #[query(...)] 属性
            for attr in &field.attrs {
                if !attr.path().is_ident("query") {
                    continue;
                }

                let res = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("eq") {
                        query_ops.push("eq".to_string());
                    } else if meta.path.is_ident("gt") {
                        query_ops.push("gt".to_string());
                    } else if meta.path.is_ident("lt") {
                        query_ops.push("lt".to_string());
                    } else if meta.path.is_ident("like") {
                        query_ops.push("like".to_string());
                    } else if meta.path.is_ident("between") {
                        query_ops.push("between".to_string());
                    } else if meta.path.is_ident("field") {
                        let val: syn::LitStr = meta.value()?.parse()?;
                        rename = val.value();
                    } else {
                        return Err(meta.error("Unsupported #[query(...)] attribute"));
                    }
                    Ok(())
                });

                if let Err(e) = res {
                    return e.to_compile_error().into();
                }
            }

            // 如果没有 query 指令，跳过字段
            if query_ops.is_empty() {
                continue;
            }

            // 生成每个操作对应的匹配逻辑
            for op in query_ops {
                let handler = match op.as_str() {
                    "eq" => quote! {
                        if let Some(val) = &self.#field_ident {
                            doc.insert(#rename, mongodb::bson::to_bson(val).unwrap());
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
                    "like" => quote! {
                        if let Some(val) = &self.#field_ident {
                            doc.insert(#rename, doc! { "$regex": val, "$options": "i" });
                        }
                    },
                    "between" => quote! {
                        if let Some(val) = &self.#field_ident {
                            doc.insert(#rename, doc! { "$gte": val, "$lte": val });
                        }
                    },
                    _ => {
                        let err = syn::Error::new(Span::call_site(), format!("Unknown query type: {}", op));
                        return err.to_compile_error().into();
                    }
                };
                field_handlers.push(handler);
            }
        }
    }

    let expanded = quote! {
        impl #struct_name {
            pub fn to_query_doc(&self) -> mongodb::bson::Document {
                use mongodb::bson::{doc, Bson};
                let mut doc = doc! {};
                #(#field_handlers)*
                doc
            }
        }
    };

    expanded.into()
}
