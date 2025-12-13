use async_sqlite::Pool;
use rand::{Rng, distributions::Alphanumeric};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: i64,
    pub did: String,
    pub url: String,
    pub secret: String,
    pub events: String, // comma-separated or "*"
    pub active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Webhook {
    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    pub fn masked_secret(&self) -> String {
        let len = self.secret.len();
        if len <= 4 {
            return "****".to_string();
        }
        let suffix = &self.secret[len - 4..];
        format!("****{}", suffix)
    }
}

pub fn generate_secret() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect()
}

pub async fn get_user_webhooks(
    pool: &Pool,
    did: &str,
) -> Result<Vec<Webhook>, async_sqlite::Error> {
    let did = did.to_string();
    pool.conn(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, did, url, secret, events, COALESCE(active, 1), created_at, updated_at FROM webhooks WHERE did = ?1 ORDER BY id DESC",
        )?;
        let iter = stmt.query_map([&did], |row| {
            Ok(Webhook {
                id: row.get(0)?,
                did: row.get(1)?,
                url: row.get(2)?,
                secret: row.get(3)?,
                events: row.get(4)?,
                active: row.get::<_, Option<bool>>(5)?.unwrap_or(true),
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;
        let mut v = Vec::new();
        for item in iter {
            v.push(item?);
        }
        Ok(v)
    })
    .await
}

pub async fn create_webhook(
    pool: &Pool,
    did: &str,
    url: &str,
    secret_opt: Option<&str>,
    events: Option<&str>,
) -> Result<(i64, String), async_sqlite::Error> {
    let secret = secret_opt.unwrap_or(&generate_secret()).to_string();
    let now = Webhook::now();
    let did_owned = did.to_string();
    let url_owned = url.to_string();
    let events_owned = events.unwrap_or("*").to_string();
    let secret_for_insert = secret.clone();

    let id = pool
        .conn(move |conn| {
            conn.execute(
                "INSERT INTO webhooks (did, url, secret, events, active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)",
                (&did_owned, &url_owned, &secret_for_insert, &events_owned, now, now),
            )?;
            Ok(conn.last_insert_rowid())
        })
        .await?;
    Ok((id, secret))
}

pub async fn update_webhook(
    pool: &Pool,
    did: &str,
    id: i64,
    url: Option<&str>,
    events: Option<&str>,
    active: Option<bool>,
) -> Result<(), async_sqlite::Error> {
    let now = Webhook::now();
    let did_owned = did.to_string();
    let url_owned = url.map(|s| s.to_string());
    let events_owned = events.map(|s| s.to_string());
    pool.conn(move |conn| {
        // Ensure ownership
        let mut check = conn.prepare("SELECT COUNT(*) FROM webhooks WHERE id = ?1 AND did = ?2")?;
        let count: i64 = check.query_row((id, &did_owned), |row| row.get(0))?;
        if count == 0 {
            return Ok(0);
        }

        // Build dynamic update
        let mut fields = Vec::new();
        if url_owned.is_some() {
            fields.push("url = ?");
        }
        if events_owned.is_some() {
            fields.push("events = ?");
        }
        if active.is_some() {
            fields.push("active = ?");
        }
        fields.push("updated_at = ?");
        let sql = format!(
            "UPDATE webhooks SET {} WHERE id = ? AND did = ?",
            fields.join(", ")
        );

        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<Box<dyn async_sqlite::rusqlite::ToSql>> = Vec::new();
        if let Some(u) = url_owned {
            params.push(Box::new(u));
        }
        if let Some(e) = events_owned {
            params.push(Box::new(e));
        }
        if let Some(a) = active {
            params.push(Box::new(a));
        }
        params.push(Box::new(now));
        params.push(Box::new(id));
        params.push(Box::new(did_owned));

        let params_ref: Vec<&dyn async_sqlite::rusqlite::ToSql> =
            params.iter().map(|b| &**b).collect();
        let _ = stmt.execute(params_ref.as_slice())?;
        Ok(1)
    })
    .await?;
    Ok(())
}

pub async fn rotate_webhook_secret(
    pool: &Pool,
    did: &str,
    id: i64,
) -> Result<String, async_sqlite::Error> {
    let new_secret = generate_secret();
    let now = Webhook::now();
    let did_owned = did.to_string();
    let new_for_update = new_secret.clone();
    pool.conn(move |conn| {
        let mut stmt = conn.prepare(
            "UPDATE webhooks SET secret = ?1, updated_at = ?2 WHERE id = ?3 AND did = ?4",
        )?;
        let _ = stmt.execute((&new_for_update, now, id, &did_owned))?;
        Ok(())
    })
    .await?;
    Ok(new_secret)
}

pub async fn delete_webhook(pool: &Pool, did: &str, id: i64) -> Result<(), async_sqlite::Error> {
    let did_owned = did.to_string();
    pool.conn(move |conn| {
        let mut stmt = conn.prepare("DELETE FROM webhooks WHERE id = ?1 AND did = ?2")?;
        let _ = stmt.execute((id, &did_owned))?;
        Ok(())
    })
    .await?;
    Ok(())
}
