use async_sqlite::Pool;

use super::models::UserPreferences;

/// Get the most frequently used emojis from all statuses
pub async fn get_frequent_emojis(
    pool: &Pool,
    limit: usize,
) -> Result<Vec<String>, async_sqlite::Error> {
    pool.conn(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT emoji, COUNT(*) as count 
             FROM status 
             GROUP BY emoji 
             ORDER BY count DESC 
             LIMIT ?1",
        )?;

        let emoji_iter = stmt.query_map([limit], |row| row.get::<_, String>(0))?;

        let mut emojis = Vec::new();
        for emoji in emoji_iter {
            emojis.push(emoji?);
        }

        Ok(emojis)
    })
    .await
}

/// Get user preferences for a given DID
pub async fn get_user_preferences(
    pool: &Pool,
    did: &str,
) -> Result<UserPreferences, async_sqlite::Error> {
    let did = did.to_string();
    pool.conn(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT did, font_family, accent_color, updated_at 
             FROM user_preferences 
             WHERE did = ?1",
        )?;

        let result = stmt.query_row([&did], |row| {
            Ok(UserPreferences {
                did: row.get(0)?,
                font_family: row.get(1)?,
                accent_color: row.get(2)?,
                updated_at: row.get(3)?,
            })
        });

        match result {
            Ok(prefs) => Ok(prefs),
            Err(async_sqlite::rusqlite::Error::QueryReturnedNoRows) => {
                // Return default preferences for new users
                Ok(UserPreferences {
                    did: did.clone(),
                    ..Default::default()
                })
            }
            Err(e) => Err(e),
        }
    })
    .await
}

/// Save user preferences
pub async fn save_user_preferences(
    pool: &Pool,
    prefs: &UserPreferences,
) -> Result<(), async_sqlite::Error> {
    let prefs = prefs.clone();
    pool.conn(move |conn| {
        conn.execute(
            "INSERT OR REPLACE INTO user_preferences (did, font_family, accent_color, updated_at) 
             VALUES (?1, ?2, ?3, ?4)",
            (
                &prefs.did,
                &prefs.font_family,
                &prefs.accent_color,
                &prefs.updated_at,
            ),
        )?;
        Ok(())
    })
    .await
}
