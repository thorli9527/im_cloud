use crate::index_trait::MongoIndexModelProvider;

pub trait HasMongoIndexes {
    fn maybe_index_models() -> Option<Vec<mongodb::IndexModel>> {
        None
    }
}
impl<T> HasMongoIndexes for T
where T: MongoIndexModelProvider
{
    fn maybe_index_models() -> Option<Vec<mongodb::IndexModel>> {
        Some(T::index_models())
    }
}
