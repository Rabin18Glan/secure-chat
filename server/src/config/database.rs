use crate::config::parameter;
use async_trait::async_trait;
use mongodb::{error::Error, Client, Database as MongoDatabase};

#[derive(Clone)]
pub struct Database {
    #[allow(dead_code)]
    client: Client,
    db: MongoDatabase,
}

#[async_trait]
pub trait DatabaseTrait {
    async fn init() -> Result<Self, Error>
    where
        Self: Sized;
    fn get_database(&self) -> &MongoDatabase;
    #[allow(dead_code)]
    fn get_client(&self) -> &Client;
}

#[async_trait]
impl DatabaseTrait for Database {
    async fn init() -> Result<Self, Error> {
        let mongo_url = parameter::get_or("MONGO_URL", "mongodb://localhost:27017");
        let db_name = parameter::get_or("DB_NAME", "chat");

        let client = Client::with_uri_str(&mongo_url).await?;
        let db = client.database(&db_name);

        Ok(Self { client, db })
    }

    fn get_database(&self) -> &MongoDatabase {
        &self.db
    }

    fn get_client(&self) -> &Client {
        &self.client
    }
}
