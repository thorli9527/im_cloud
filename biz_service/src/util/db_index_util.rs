use bson::Document;
use common::util::common_utils::hash_index;
use deadpool_redis::redis::AsyncCommands;
use futures_util::StreamExt;
use mongodb::{options, Collection, IndexModel};
use std::clone;
use std::collections::HashMap;
use std::ops::Index;

pub async fn index_create(coll: Collection<Document>, target_list: Vec<IndexModel>) {
    let mut cursor = coll.list_indexes().await.unwrap();
    let mut create_index_list = vec![];
    for target in target_list {
        let options = target.options.clone().unwrap();
        let name = options.name.unwrap();
        let mut has_index = false;
        while let Some(index) = cursor.next().await {
            match index {
                Ok(index_info) => {
                    let index_name = index_info.options.clone().unwrap().name.unwrap();
                    if name == index_name {
                        has_index = true;
                        break;
                    }
                }
                Err(e) => log::error!("❌ 列出索引失败: {:?}", e),
            }
        }
        if !has_index {
            create_index_list.push(target);
        }
    }

    if !create_index_list.is_empty() {
        for target in create_index_list {
            match coll.create_index(target.clone()).await {
                Ok(_) => log::info!("✅ 创建索引成功: {}", target.keys.to_string()),
                Err(e) => log::error!("❌ 创建索引失败: {:?}", e),
            }
        }
    }
}
