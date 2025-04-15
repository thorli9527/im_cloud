use crate::entitys::path_entity::PathInfo;
use common::config::ServerRes;
use common::repository_util::BaseRepository;
use crate::biz_services::path_service::PathService;
use crate::biz_services::role_service::RoleService;
use crate::entitys::menu_entity::MenuInfo;
use crate::entitys::role_entity::RoleInfo;
use crate::entitys::user_role_entity::UserRole;

pub struct UserRoleService {
    pub dao: BaseRepository<UserRole>,
}

impl UserRoleService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection("user_role");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
