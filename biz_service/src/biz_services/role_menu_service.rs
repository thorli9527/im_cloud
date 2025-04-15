use crate::entitys::path_entity::PathInfo;
use common::config::ServerRes;
use common::repository_util::BaseRepository;
use crate::biz_services::path_service::PathService;
use crate::biz_services::role_service::RoleService;
use crate::entitys::menu_entity::MenuInfo;
use crate::entitys::role_entity::RoleInfo;
use crate::entitys::role_menu_entity::RoleMenuRel;

pub struct RoleMenuRelService {
    pub dao: BaseRepository<RoleMenuRel>,
}

impl RoleMenuRelService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection("role_menu_rel");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
