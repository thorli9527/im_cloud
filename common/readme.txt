use query_macro::QueryFilter;
use serde::{Serialize, Deserialize};

#[derive(QueryFilter, Serialize, Deserialize, Debug)]
pub struct AdvancedQuery {
    #[query(eq)]
    pub username: Option<String>,

    #[query(between)]
    pub age: Option<u32>,

    #[query(in)]
    pub roles: Option<Vec<String>>,

    #[query(exists)]
    pub deleted: Option<bool>,

    #[query(regex, field = "nickname")]
    pub nick_like: Option<String>,
}

fn main() {
    let query = AdvancedQuery {
        username: Some("thorli".into()),
        age: Some(30),
        roles: Some(vec!["admin".into(), "editor".into()]),
        deleted: Some(false),
        nick_like: Some("^T".into()),
    };

    let doc = query.to_query_doc();
    println!("📄 Mongo 查询: {:?}", doc);
}