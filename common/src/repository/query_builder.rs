use crate::repository_util::OrderType;
use mongodb::bson::{doc, Bson, Document};
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Default,Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub index: i32,
    pub page_size: i32,
    pub order_column:String,
    pub order_type:OrderType,
}
#[derive(Debug, Serialize, Deserialize, Default,Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryBuilder {
    clauses: Vec<Document>,
    current: Document,
    logic_op: Option<String>,
    pub limit: Option<i64>,
    pub skip: Option<i64>,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn eq(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.current.insert(field, value.into());
        self
    }

    pub fn gt(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.current.insert(field, doc! { "$gt": value.into() });
        self
    }

    pub fn lt(mut self, field: &str, value: impl Into<Bson>) -> Self {
        self.current.insert(field, doc! { "$lt": value.into() });
        self
    }

    pub fn in_array<T: Into<Bson>>(mut self, field: &str, values: Vec<T>) -> Self {
        let arr = values.into_iter().map(Into::into).collect::<Vec<_>>();
        self.current.insert(field, doc! { "$in": arr });
        self
    }

    pub fn exists(mut self, field: &str) -> Self {
        self.current.insert(field, doc! { "$exists": true });
        self
    }

    pub fn not_exists(mut self, field: &str) -> Self {
        self.current.insert(field, doc! { "$exists": false });
        self
    }

    pub fn and(mut self) -> Self {
        self.logic_op = Some("$and".into());
        self.clauses.push(self.current);
        self.current = Document::new();
        self
    }

    pub fn or(mut self) -> Self {
        self.logic_op = Some("$or".into());
        self.clauses.push(self.current);
        self.current = Document::new();
        self
    }

    pub fn set_limit(mut self, val: i64) -> Self {
        self.limit = Some(val);
        self
    }

    pub fn set_skip(mut self, val: i64) -> Self {
        self.skip = Some(val);
        self
    }

    pub fn build(mut self) -> Document {
        if !self.current.is_empty() {
            self.clauses.push(self.current);
        }
        match self.logic_op.as_deref() {
            Some("$and") => doc! { "$and": self.clauses },
            Some("$or") => doc! { "$or": self.clauses },
            _ if self.clauses.len() == 1 => self.clauses.remove(0),
            _ => doc! {},
        }
    }
}