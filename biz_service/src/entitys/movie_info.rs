use mongodb::bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieInfo {
    #[serde(rename = "_id")]
    id: String,
    name:String,
    time:i32,
    pub create_time: DateTime,
}

pub struct MovieItem{
    #[serde(rename = "_id")]
    id: String,
    movie_id:String,
    time:i32,
    file_id:String,
}
