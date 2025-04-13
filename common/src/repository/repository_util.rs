use crate::config::ServerRes;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use mongodb::{bson::Document, error::Result, Collection};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::cmp::PartialEq;
use std::marker::PhantomData;
use mongodb::bson::oid::ObjectId;
use crate::errors::AppError;

pub struct PageResult<T> {
    pub items: Vec<T>,
    pub has_next: bool,
    pub has_prev: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum OrderType {
    #[default]
    Asc,
    Desc,
}
#[async_trait]
pub trait Repository<T> {
    async fn find_by_id(&self, id: String) -> Result<Option<T>>;
    async fn insert(&self, entity: &T) -> Result<()>;
    async fn find_one(&self, filter: Document) -> Result<Option<T>>;
    async fn query_all(&self) -> Result<Vec<T>>;
    async fn update(&self, filter: Document, update: Document) -> Result<u64>;
    async fn delete(&self, filter: Document) -> Result<u64>;
    async fn query_by_page(&self, filter: Document, page_size: i64, order_type: Option<OrderType>, sort_field: &str) -> Result<PageResult<T>>;
}

#[allow(dead_code)]
pub struct BaseRepository<T: Send + Sync> {
    pub collection: Collection<T>, // 线程安全的数据库连接池
    pub db_res: ServerRes,
    _marker: PhantomData<T>,
}

impl<T: Send + Sync> BaseRepository<T> {
    pub fn new(db_res: ServerRes, collection: Collection<T>) -> Self {
        Self { collection, db_res, _marker: Default::default() }
    }
}

#[async_trait]
impl<T: Send + Sync> Repository<T> for BaseRepository<T>
where
    T: Serialize + DeserializeOwned + Unpin + Send + Sync,
{
    async fn find_by_id(&self, id: String) -> Result<Option<T>> {
        let obj_id = ObjectId::parse_str(id).unwrap();

        let option = self.find_one(doc! { "_id": obj_id }).await?;
        Ok(option)
    }

    async fn insert(&self, entity: &T) -> Result<()> {
        self.collection.insert_one(entity).await?;
        Ok(())
    }

    async fn find_one(&self, filter: Document) -> Result<Option<T>> {
        let result = self.collection.find_one(filter).await?;
        Ok(result)
    }

    async fn query_all(&self) -> Result<Vec<T>> {
        let mut cursor = self.collection.find(doc! {}).await?;
        let mut result = vec![];
        while let Some(doc) = cursor.try_next().await? {
            result.push(doc);
        }
        Ok(result)
    }

    async fn update(&self, filter: Document, update: Document) -> Result<u64> {
        let result = self.collection.update_many(filter, update).await?;
        Ok(result.modified_count)
    }

    async fn delete(&self, filter: Document) -> Result<u64> {
        let result = self.collection.delete_many(filter).await?;
        Ok(result.deleted_count)
    }

    async fn query_by_page(&self, filter: Document, page_size: i64, order_type: Option<OrderType>, sort_field: &str) -> Result<PageResult<T>> {
        let mut sort_direction = 0;
        match order_type {
            None => sort_direction = 1,
            Some(order) => {
                if (order == OrderType::Asc) {
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
            results.push(doc);
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
