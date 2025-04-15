use crate::biz_services::ServerRes;
use crate::entitys::file_entity::FileInfo;
use common::repository_util::BaseRepository;

pub struct FileService {
    pub dao: BaseRepository<FileInfo>,
}

impl FileService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection("file_info");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
