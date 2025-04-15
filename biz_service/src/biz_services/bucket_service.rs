use crate::biz_services::ServerRes;
use crate::entitys::bucket_entity::BucketInfo;
use common::repository_util::BaseRepository;

pub struct BucketService {
    dao: BaseRepository<BucketInfo>,
}

impl BucketService {
    pub fn new(db_res:ServerRes) -> Self {
        let collection = db_res.db.collection("bucket_info");
        Self { dao: BaseRepository::new(db_res, collection.clone()) }
    }
}
