use crate::db::StatusFromDb;
use crate::lexicons;
use crate::lexicons::xyz::statusphere::Status;
use anyhow::anyhow;
use async_sqlite::Pool;
use async_trait::async_trait;
use atrium_api::types::Collection;
use log::error;
use rocketman::{
    connection::JetstreamConnection,
    handler,
    ingestion::LexiconIngestor,
    options::JetstreamOptions,
    types::event::{Event, Operation},
};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[async_trait]
impl LexiconIngestor for StatusSphereIngester {
    async fn ingest(&self, message: Event<Value>) -> anyhow::Result<()> {
        if let Some(commit) = &message.commit {
            //We manually construct the uri since Jetstream does not provide it
            //at://{users did}/{collection: xyz.statusphere.status}{records key}
            let record_uri = format!("at://{}/{}/{}", message.did, commit.collection, commit.rkey);
            match commit.operation {
                Operation::Create | Operation::Update => {
                    if let Some(record) = &commit.record {
                        let status_at_proto_record = serde_json::from_value::<
                            lexicons::xyz::statusphere::status::RecordData,
                        >(record.clone())?;

                        if let Some(ref _cid) = commit.cid {
                            // Although esquema does not have full validation yet,
                            // if you get to this point,
                            // You know the data structure is the same
                            let created = status_at_proto_record.created_at.as_ref();
                            let right_now = chrono::Utc::now();
                            // We save or update the record in the db
                            StatusFromDb {
                                uri: record_uri,
                                author_did: message.did.clone(),
                                status: status_at_proto_record.status.clone(),
                                created_at: created.to_utc(),
                                indexed_at: right_now,
                                handle: None,
                            }
                            .save_or_update(&self.db_pool)
                            .await?;
                        }
                    }
                }
                Operation::Delete => StatusFromDb::delete_by_uri(&self.db_pool, record_uri).await?,
            }
        } else {
            return Err(anyhow!("Message has no commit"));
        }
        Ok(())
    }
}
pub struct StatusSphereIngester {
    db_pool: Arc<Pool>,
}

pub async fn start_ingester(db_pool: Arc<Pool>) {
    // init the builder
    let opts = JetstreamOptions::builder()
        // your EXACT nsids
        // Which in this case is xyz.statusphere.status
        .wanted_collections(vec![Status::NSID.parse().unwrap()])
        .build();
    // create the jetstream connector
    let jetstream = JetstreamConnection::new(opts);

    // create your ingesters
    // Which in this case is xyz.statusphere.status
    let mut ingesters: HashMap<String, Box<dyn LexiconIngestor + Send + Sync>> = HashMap::new();
    ingesters.insert(
        // your EXACT nsid
        Status::NSID.parse().unwrap(),
        Box::new(StatusSphereIngester { db_pool }),
    );

    // tracks the last message we've processed
    let cursor: Arc<Mutex<Option<u64>>> = Arc::new(Mutex::new(None));

    // get channels
    let msg_rx = jetstream.get_msg_rx();
    let reconnect_tx = jetstream.get_reconnect_tx();

    // spawn a task to process messages from the queue.
    // this is a simple implementation, you can use a more complex one based on needs.
    let c_cursor = cursor.clone();
    tokio::spawn(async move {
        while let Ok(message) = msg_rx.recv_async().await {
            if let Err(e) =
                handler::handle_message(message, &ingesters, reconnect_tx.clone(), c_cursor.clone())
                    .await
            {
                error!("Error processing message: {}", e);
            };
        }
    });

    // connect to jetstream
    // retries internally, but may fail if there is an extreme error.
    if let Err(e) = jetstream.connect(cursor.clone()).await {
        error!("Failed to connect to Jetstream: {}", e);
        std::process::exit(1);
    }
}
