use crate::entitys::menu_entity::MenuEntity;
use anyhow::Result;
use common::repository_util::{BaseRepository, Repository};
use common::util::date_util::now;
use mongodb::{bson::doc, Database};
use once_cell::sync::OnceCell;
use std::sync::Arc;
#[derive(Debug)]
pub struct MenuService {
    dao: BaseRepository<MenuEntity>,
}

impl MenuService {
    /// 初始化新的 MenuService，传入数据库引用
    pub fn new(db: Database) -> Self {
        let collection = db.collection("menu");
        Self { dao: BaseRepository::new(db, collection) }
    }

    /// 获取所有菜单列表（从数据库查询后按 order 字段升序排序）
    pub async fn get_all_menus(&self) -> Result<Vec<MenuEntity>> {
        let mut menus = self.dao.query(doc! {}).await?;  // 查询所有菜单
        // 根据 order 字段排序（值小的在前）
        menus.sort_by(|a, b| a.order.cmp(&b.order));
        Ok(menus)
    }

    /// 添加新菜单项，返回插入的菜单ID
    pub async fn add_menu(&self, title: &str, parent_id: Option<String>, path: &str,
                          component: Option<String>, icon: Option<String>,
                          permission: Option<String>, hidden: bool, order: i32) -> Result<String> {
        let entity = MenuEntity {
            id: "".into(),  // 让数据库自动生成ID，如使用 ObjectId
            title: title.into(),
            parent_id,
            path: path.into(),
            component,
            icon,
            permission,
            hidden,
            order,
            create_time: now() as u64,
        };
        // 插入数据库
        self.dao.insert(&entity).await  // 返回生成的ID
    }

    // 单例模式相关: 初始化和获取全局实例
    pub fn init(db: Database) {
        INSTANCE.set(Arc::new(Self::new(db)))
            .expect("MenuService already initialized");
    }
    pub fn get() -> Arc<Self> {
        INSTANCE.get().expect("MenuService not initialized").clone()
    }
}

static INSTANCE: OnceCell<Arc<MenuService>> = OnceCell::new();
