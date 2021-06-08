use crate::chain_api::{RewardSlash, RewardSlashPage, Transfer, TransferPage};
use crate::{Context, Result};
use bson::{doc, from_document, to_bson, to_document, Bson, Document};
use mongodb::{Client, Database as MongoDb};
use serde::Serialize;
use std::borrow::Cow;

const TRANSFER_EVENTS_RAW: &'static str = "events_transfer_raw";
const REWARD_SLASH_EVENTS_RAW: &'static str = "events_transfer_raw";

/// Convenience trait. Converts a value to BSON.
trait ToBson {
    fn to_bson(&self) -> Result<Bson>;
    fn to_document(&self) -> Result<Document>;
}

impl<T: Serialize> ToBson for T {
    fn to_bson(&self) -> Result<Bson> {
        Ok(to_bson(self)?)
    }
    fn to_document(&self) -> Result<Document> {
        Ok(to_document(self)?)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ContextData<'a, T: Clone> {
    context: Cow<'a, Context>,
    data: Cow<'a, T>,
}

pub struct Database {
    db: MongoDb,
}

impl Database {
    pub async fn new(uri: &str, db: &str) -> Result<Self> {
        let db = Client::with_uri_str(uri).await?.database(db);

        // Currently, the Rust MongoDb driver does not support indexing
        // natively. This is the current workaround. See
        // https://github.com/mongodb/mongo-rust-driver/pull/188
        db.run_command(
            doc! {
                "createIndexes": TRANSFER_EVENTS_RAW.to_bson()?,
                "indexes": [
                    {
                        "key": { "data.extrinsic_index": 1 },
                        "name": "transfer_extrinsic_id_constraint",
                        "unique": true
                    },
                ]
            },
            None,
        )
        .await?;

        Ok(Database { db: db })
    }
    pub async fn store_transfer_event(
        &self,
        context: &Context,
        event: &TransferPage,
    ) -> Result<usize> {
        let coll = self
            .db
            .collection::<ContextData<Transfer>>(TRANSFER_EVENTS_RAW);

        // Add the full context to each transfer, so the corresponding account
        // can be identified.
        let transfers: Vec<ContextData<Transfer>> = event
            .transfers
            .iter()
            .map(|t| ContextData {
                context: Cow::Borrowed(context),
                data: Cow::Borrowed(t),
            })
            .collect();

        coll.insert_many(transfers, None).await?;

        Ok(0)
    }
    pub async fn store_reward_slash_event(
        &self,
        context: &Context,
        event: &RewardSlashPage,
    ) -> Result<usize> {
        let coll = self
            .db
            .collection::<ContextData<RewardSlash>>(REWARD_SLASH_EVENTS_RAW);

        let reward_slashes: Vec<ContextData<RewardSlash>> = event
            .list
            .iter()
            .map(|rs| ContextData {
                context: Cow::Borrowed(context),
                data: Cow::Borrowed(rs),
            })
            .collect();

        coll.insert_many(reward_slashes, None).await?;

        Ok(0)
    }
}
