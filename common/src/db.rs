use mongodb::{Client, Database, options::ClientOptions};

pub async fn init_db(url: &str, db_name: &str) -> mongodb::error::Result<Database> {
    let options = ClientOptions::parse(url).await?;
    let client = Client::with_options(options)?;
    Ok(client.database(db_name))
}
