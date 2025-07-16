extern crate proc_macro;

mod mongo_index_macro;
mod mongo_query_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MongoIndexModelProvider, attributes(mongo_index))]
pub fn mongo_index_model_provider(input: TokenStream) -> TokenStream {
    mongo_index_macro::expand_index_model_provider(input)
}



#[proc_macro_derive(QueryFilter, attributes(query))]
pub fn derive_query_filter(input: TokenStream) -> TokenStream {
    mongo_query_macro::derive_query_filter(input)
}