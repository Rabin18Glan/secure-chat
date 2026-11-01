use crate::config::parameter;
use async_trait::async_trait;
use bson::doc;
use mongodb::options::{ClientOptions, IndexOptions};
use mongodb::{error::Error, Client, Database as MongoDatabase, IndexModel};

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

        let mut options = ClientOptions::parse(&mongo_url).await?;
        options.server_selection_timeout = Some(std::time::Duration::from_secs(5));
        options.connect_timeout = Some(std::time::Duration::from_secs(5));
        let client = Client::with_options(options)?;
        let db = client.database(&db_name);
        db.run_command(doc! {"ping": 1}).await?;
        let unique = || IndexOptions::builder().unique(true).build();
        db.collection::<bson::Document>("users")
            .create_indexes([
                IndexModel::builder()
                    .keys(doc! {"username": 1})
                    .options(unique())
                    .build(),
                IndexModel::builder()
                    .keys(doc! {"email": 1})
                    .options(unique())
                    .build(),
                IndexModel::builder()
                    .keys(doc! {"googleId": 1})
                    .options(
                        IndexOptions::builder()
                            .unique(true)
                            .partial_filter_expression(doc! {"googleId": {"$type": "string"}})
                            .build(),
                    )
                    .build(),
            ])
            .await?;
        db.collection::<bson::Document>("sessions")
            .create_index(
                IndexModel::builder()
                    .keys(doc! {"sessionId": 1})
                    .options(unique())
                    .build(),
            )
            .await?;
        db.collection::<bson::Document>("messages")
            .create_index(
                IndexModel::builder()
                    .keys(doc! {"users": 1, "updatedAt": 1, "_id": 1})
                    .build(),
            )
            .await?;

        Ok(Self { client, db })
    }

    fn get_database(&self) -> &MongoDatabase {
        &self.db
    }

    fn get_client(&self) -> &Client {
        &self.client
    }
}
