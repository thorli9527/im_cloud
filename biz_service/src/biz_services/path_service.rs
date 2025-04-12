use crate::entitys::path_entity::PathInfo;
use common::config::ServerRes;
use common::repository_util::BaseRepository;
pub struct PathService {
    pub dao: BaseRepository<PathInfo>,
}

impl PathService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection::<PathInfo>("path_info");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
