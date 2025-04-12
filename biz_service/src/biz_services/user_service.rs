use crate::entitys::user_entity::UserInfo;
use common::config::ServerRes;
use common::repository_util::BaseRepository;

pub struct UserService {
    pub dao: BaseRepository<UserInfo>,
}

impl UserService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection::<UserInfo>("user_info");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
