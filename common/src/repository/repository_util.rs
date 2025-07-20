use anyhow::Result;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, Bson};
use mongodb::options::FindOptions;
use mongodb::{bson, bson::Document, Collection, Database};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::cmp::PartialEq;
use std::marker::PhantomData;
use utoipa::ToSchema;
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub has_next: bool,
    pub has_prev: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, ToSchema)]
pub enum OrderType {
    #[default]
    Asc,
    Desc,
}
#[async_trait]
pub trait Repository<T> {
    async fn find_by_id(&self, id: &str) -> Result<Option<T>>;
    async fn insert(&self, entity: &T) -> Result<String>;
    async fn insert_many(&self, entities: &[T]) -> Result<()>;
    async fn find_one(&self, filter: Document) -> Result<Option<T>>;
    async fn query_all(&self) -> Result<Vec<T>>;
    async fn query(&self, filter: Document) -> Result<Vec<T>>;
    async fn save(&self, entity: &T) -> Result<()>;
    async fn un_set(&self, id: &str, property: &str) -> Result<()>;
    async fn update(&self, filter: Document, update: Document) -> Result<u64>;
    async fn up_property<E: Send + Sync + Serialize>(&self, id: &str, property: &str, value: E) -> Result<()>;
    async fn delete(&self, filter: Document) -> Result<u64>;
    async fn delete_by_id(&self, id: &str) -> Result<u64>;
    async fn query_by_page(&self, filter: Document, page_size: i64, order_type: Option<OrderType>, sort_field: &str) -> Result<PageResult<T>>;
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BaseRepository<T: Send + Sync> {
    pub collection: Collection<Document>, // 线程安全的数据库连接池
    pub db: Database,
    _marker: PhantomData<T>,
    
}

impl<T: Send + Sync> BaseRepository<T> {
    pub fn new(db: Database, collection: Collection<Document>) -> Self {
        Self { collection, db, _marker: Default::default() }
    }
    
}

#[async_trait]
impl<T: Send + Sync + std::fmt::Debug> Repository<T> for BaseRepository<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync,
{
    async fn find_by_id(&self, id: &str) -> Result<Option<T>> {
        let obj_id = ObjectId::parse_str(id)?;
        let option = self.find_one(doc! { "_id": obj_id }).await?;
        Ok(option)
    }

    async fn insert(&self, entity: &T) -> Result<String> {
        let mut doc = bson::to_document(entity)?;
        if let Some(bson::Bson::String(id_str)) = doc.get_str("id").ok().map(str::to_string).map(bson::Bson::String) {
            if let Ok(object_id) = ObjectId::parse_str(&id_str) {
                doc.insert("_id", object_id);
            }
        }
        doc.remove("id");
        let result = self.collection.insert_one(doc).await?;
        if let Bson::ObjectId(oid) = result.inserted_id {
            Ok(oid.to_string())
        } else {
            Err(anyhow::anyhow!("❌ 插入成功但未返回 ObjectId"))
        }
    }

    async fn insert_many(&self, entities: &[T]) -> Result<()> {
        if entities.is_empty() {
            return Ok(());
        }

        let mut docs = Vec::with_capacity(entities.len());

        for entity in entities {
            let mut doc = bson::to_document(entity)?;

            if let Some(Bson::String(id_str)) = doc.get_str("id").ok().map(str::to_string).map(Bson::String) {
                if let Ok(object_id) = ObjectId::parse_str(&id_str) {
                    doc.insert("_id", object_id);
                }
            }

            doc.remove("id");
            docs.push(doc);
        }

        self.collection.insert_many(docs).await?;
        Ok(())
    }

    async fn find_one(&self, filter: Document) -> Result<Option<T>> {
        if let Some(doc) = self.collection.find_one(filter).await? {
            let deserialized = bson::from_document(transform_doc_id(doc))?;
            Ok(Some(deserialized))
        } else {
            Ok(None)
        }
    }

    async fn query_all(&self) -> Result<Vec<T>> {
        let mut cursor = self.collection.find(doc! {}).await?;
        let mut result = Vec::<T>::new();
        while let Some(doc) = cursor.try_next().await? {
            result.push(bson::from_document(transform_doc_id(doc))?);
        }
        Ok(result)
    }

    async fn query(&self, filter: Document) -> Result<Vec<T>> {
        let mut cursor = self.collection.find(filter).await?;
        let mut results: Vec<T> = vec![];
        while let Some(doc) = cursor.try_next().await? {
            results.push(bson::from_document(transform_doc_id(doc))?);
        }
        return Ok(results);
    }

    async fn save(&self, entity: &T) -> Result<()> {
        let mut doc = bson::to_document(&entity)?;
        let id = build_id(&doc);
        doc.remove("id");
        let object_id = ObjectId::parse_str(id)?; // 将字符串转为 ObjectId
        let filter = doc! { "_id": object_id };
        let _ = self.collection.update_one(filter, doc).await?;
        Ok(())
    }

    async fn un_set(&self, id: &str, property: &str) -> Result<()> {
        let object_id = ObjectId::parse_str(id)?;
        let filter = doc! {"_id":object_id};
        let update = doc! {
            "$unset": {
                property: ""
            }
        };
        self.collection.update_one(filter, update).await?;
        return Ok(());
    }

    async fn update(&self, filter: Document, update: Document) -> Result<u64> {
        let result = self.collection.update_many(filter, update).await?;
        Ok(result.modified_count)
    }

    async fn up_property<E: Send + Sync + Serialize>(&self, id: &str, property: &str, value: E) -> Result<()> {
        let object_id = ObjectId::parse_str(id.clone())?;
        let filter = doc! {"_id":object_id};
        let update = doc! {
            "$set": {
                property: bson::to_bson(&value)?
            }
        };

        self.collection.update_one(filter, update).await?;
        Ok(())
    }

    async fn delete(&self, filter: Document) -> Result<u64> {
        let result = self.collection.delete_many(filter).await?;
        Ok(result.deleted_count)
    }

    async fn delete_by_id(&self, id: &str) -> Result<u64> {
        let object_id = ObjectId::parse_str(id).unwrap();
        let result = self.collection.delete_many(doc! {"_id":object_id}).await?;
        Ok(result.deleted_count)
    }

    async fn query_by_page(&self, filter: Document, page_size: i64, order_type: Option<OrderType>, sort_field: &str) -> Result<PageResult<T>> {
        let mut sort_direction = 0;
        match order_type {
            None => sort_direction = 1,
            Some(order) => {
                if order == OrderType::Asc {
                    sort_direction = 1
                }
                if order == OrderType::Desc {
                    sort_direction = -1
                }
            }
        };
        let real_limit = page_size + 1;
        let find_options = FindOptions::builder().sort(doc! { sort_field: sort_direction }).limit(real_limit).build();

        let mut cursor = self.collection.find(filter).with_options(find_options).await?;
        let mut results: Vec<T> = vec![];
        while let Some(doc) = cursor.try_next().await? {
            results.push(bson::from_document(transform_doc_id(doc))?);
        }

        if sort_direction < 0 {
            results.reverse(); // 翻页情况下逆转以保持顺序
        }
        let has_more = results.len() as i64 > page_size;
        if has_more {
            results.pop(); // 移除多出来的那条
        }
        Ok(PageResult {
            items: results,
            has_next: if sort_direction > 0 { has_more } else { true }, // 可以精细判断
            has_prev: if sort_direction < 0 { has_more } else { true },
        })
    }
}

fn transform_doc_id(mut doc: Document) -> Document {
    if let Some(Bson::ObjectId(oid)) = doc.remove("_id") {
        doc.insert("id", Bson::String(oid.to_hex()));
    }
    doc
}

fn build_id(doc: &Document) -> String {
    return doc.get("id").unwrap().to_string();
}
