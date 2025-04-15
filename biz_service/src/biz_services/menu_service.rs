use crate::entitys::path_entity::PathInfo;
use common::config::ServerRes;
use common::repository_util::BaseRepository;
use crate::biz_services::path_service::PathService;
use crate::entitys::menu_entity::MenuInfo;

pub struct MenuService {
    pub dao: BaseRepository<MenuInfo>,
}

impl MenuService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection("menu_info");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
