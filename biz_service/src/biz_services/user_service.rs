use std::collections::HashMap;
use std::io::Chain;
use actix_web::web::Data;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use crate::entitys::user_entity::UserInfo;
use common::config::ServerRes;
use common::errors::AppError;
use common::repository_util::{BaseRepository, Repository};
use log::warn;
use utoipa::ToSchema;
use crate::biz_services::menu_service::MenuService;
use crate::biz_services::{menu_service, role_menu_service};
use crate::biz_services::role_menu_service::RoleMenuRelService;
use crate::biz_services::user_role_service::UserRoleService;
use crate::entitys::menu_entity::{MenuInfo, MenuTypeEnum};
#[derive(Debug, Clone, Serialize, Deserialize, Default,ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MenuDto {
    #[serde(rename = "_id")]
   pub id: String,
    pub  menu_name: String,
    pub   path: String,
    pub icon: String,
    pub  al_icon: String,
    pub  open: bool,
    pub   parent_id: String,
    pub  code: String,
    pub  fun_type: MenuTypeEnum,
    pub new_link_flag: i32,
    pub  children: Vec<MenuDto>,
}
pub struct UserService {
    pub dao: BaseRepository<UserInfo>,
}

impl UserService {
    pub fn new(db_res: ServerRes) -> Self {
        let collection = db_res.db.collection("user_info");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
    pub async fn load_user_menu(&self,user_id: String,
                                user_role_service: Data<UserRoleService>,
                                role_menu_service: Data<RoleMenuRelService>,
                                menu_service: Data<MenuService>) -> Result<Vec<MenuDto>,AppError>{
        let user_role_list = user_role_service.dao.query(doc! { "user_id": &user_id }).await?;

        let mut menu_map: HashMap<String, MenuDto> = HashMap::new();
        for user_role in user_role_list{
            let user_menu_ref_list = role_menu_service.dao.query(doc! { "role_id": &user_role.role_id }).await?;
            for role_menu_ref in user_menu_ref_list{
                let menu_option = menu_service.dao.find_by_id(role_menu_ref.menu_id.clone()).await?;
                match menu_option {
                    Some(menu)=>{
                        let dto = MenuDto {
                            id: menu.id.clone(),
                            menu_name: menu.menu_name,
                            path: menu.path,
                            icon: menu.icon,
                            al_icon: menu.al_icon,
                            open: menu.open,
                            parent_id: menu.parent_id,
                            code: menu.code,
                            fun_type: menu.fun_type,
                            new_link_flag: menu.new_link_flag,
                            children: vec![],
                        };
                        menu_map.insert(menu.id.clone(), dto);
                    },
                    None=>{
                        warn!("menu.not.found:{}",role_menu_ref.menu_id);
                    }
                }
            }
        }

        // 构建树
        let mut tree = Vec::new();
        let mut map_clone = menu_map.clone();

        for (_id, menu) in menu_map.into_iter() {
            if let Some(parent) = map_clone.get_mut(&menu.parent_id) {
                parent.children.push(menu);
            } else {
                tree.push(menu);
            }
        }
        Ok(tree)
    }
}
