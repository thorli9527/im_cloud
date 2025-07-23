use quote::quote;
use syn::{parse_macro_input, DeriveInput, Lit};

pub fn expand_index_model_provider(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;

    let mut index_exprs = vec![];

    for attr in &ast.attrs {
        if attr.path().is_ident("mongo_index") {
            let _ = attr.parse_nested_meta(|meta| {
                let mut fields = vec![];
                let mut is_unique = false;
                let mut sort_order = 1i32;

                let input = meta.input;
                while !input.is_empty() {
                    let path: syn::Path = match input.parse() {
                        Ok(p) => p,
                        Err(_) => break,
                    };

                    if path.is_ident("fields") {
                        let content;
                        syn::bracketed!(content in input);
                        while let Ok(Lit::Str(lit)) = content.parse() {
                            fields.push(lit.value());
                            let _ = content.parse::<syn::Token![,]>();
                        }
                    } else if path.is_ident("unique") {
                        is_unique = true;
                    } else if path.is_ident("order") {
                        let content;
                        syn::parenthesized!(content in input);
                        if let Ok(Lit::Str(lit)) = content.parse() {
                            if lit.value().eq_ignore_ascii_case("desc") {
                                sort_order = -1;
                            }
                        }
                    }

                    let _ = input.parse::<syn::Token![,]>();
                }

                let mut key_doc = quote! { bson::doc! {} };
                for field in &fields {
                    let field_name = field.clone();
                    key_doc = quote! {
                        {
                            let mut d = #key_doc;
                            d.insert(#field_name, #sort_order);
                            d
                        }
                    };
                }

                let options = if is_unique {
                    quote! { Some(mongodb::options::IndexOptions::builder().unique(true).build()) }
                } else {
                    quote! { None }
                };

                let model = quote! {
                    mongodb::IndexModel::builder()
                        .keys(#key_doc)
                        .options(#options)
                        .build()
                };

                index_exprs.push(model);

                Ok(())
            });
        }
    }

    let gen = quote! {
        impl MongoIndexModelProvider for #name {
            fn index_models() -> Vec<mongodb::IndexModel> {
                vec![
                    #(#index_exprs),*
                ]
            }
        }
    };

    gen.into()
}
