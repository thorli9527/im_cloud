use crate::entitys::path_entity::PathInfo;
use common::config::ServerRes;
use common::repository_util::BaseRepository;
use crate::biz_services::path_service::PathService;
use crate::entitys::menu_entity::MenuInfo;
use crate::entitys::role_entity::RoleInfo;

pub struct RoleService {
    pub dao: BaseRepository<RoleInfo>,
}

impl RoleService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection("role_info");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
