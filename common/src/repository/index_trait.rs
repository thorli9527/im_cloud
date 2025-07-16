use mongodb::IndexModel;

pub trait MongoIndexModelProvider {
    fn index_models() -> Vec<IndexModel>;
}
