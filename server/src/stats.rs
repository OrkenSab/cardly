// Copyright: Cardly contributors
// License: GNU AGPL, version 3 or later; https://www.gnu.org/licenses/agpl.html

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use axum::extract::Query;
use axum::extract::State;
use axum::http::header;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use chrono::DateTime;
use chrono::Duration;
use chrono::FixedOffset;
use chrono::NaiveDate;
use chrono::Timelike;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::auth::authenticate_request;
use crate::AppState;

const FORECAST_HORIZON_DAYS: i64 = 90;
const DEFAULT_ROLLOVER_HOUR: i64 = 4;

#[derive(Debug, Deserialize)]
pub struct HeatmapQuery {
    pub deck_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapResponse {
    pub history: BTreeMap<String, u32>,
    pub forecast: BTreeMap<String, u32>,
    pub today: String,
    pub today_count: u32,
    pub daily_average: f64,
    pub days_learned_percent: f64,
    pub current_streak: u32,
    pub longest_streak: u32,
}

fn json_err(status: StatusCode, msg: impl Into<String>) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        Json(serde_json::json!({ "error": msg.into() })),
    )
        .into_response()
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn fixed_offset_from_minutes_west(minutes_west: i32) -> FixedOffset {
    let bounded = minutes_west.clamp(-23 * 60, 23 * 60);
    FixedOffset::west_opt(bounded * 60).unwrap_or_else(|| FixedOffset::west_opt(0).unwrap())
}

fn anki_study_date(ts_secs: i64, offset: FixedOffset, rollover_hour: i64) -> NaiveDate {
    let utc_dt = DateTime::from_timestamp(ts_secs, 0).unwrap_or(DateTime::UNIX_EPOCH);
    let local_dt = utc_dt.with_timezone(&offset);
    (local_dt - Duration::hours(rollover_hour)).date_naive()
}

fn compute_today_day_index(
    crt_secs: i64,
    creation_offset: FixedOffset,
    now_secs: i64,
    current_offset: FixedOffset,
    rollover_hour: i64,
) -> i64 {
    let created_dt = DateTime::from_timestamp(crt_secs, 0)
        .unwrap_or(DateTime::UNIX_EPOCH)
        .with_timezone(&creation_offset);
    let now_dt = DateTime::from_timestamp(now_secs, 0)
        .unwrap_or(DateTime::UNIX_EPOCH)
        .with_timezone(&current_offset);

    let rollover_passed = (now_dt.hour() as i64) >= rollover_hour;
    let days = now_dt
        .date_naive()
        .signed_duration_since(created_dt.date_naive())
        .num_days();
    let adjusted = if rollover_passed { days } else { days - 1 };
    adjusted.max(0)
}

pub fn empty_heatmap_stats(today: String) -> HeatmapResponse {
    HeatmapResponse {
        history: BTreeMap::new(),
        forecast: BTreeMap::new(),
        today,
        today_count: 0,
        daily_average: 0.0,
        days_learned_percent: 0.0,
        current_streak: 0,
        longest_streak: 0,
    }
}

fn parse_config_json_value(val: ValueRef<'_>) -> Option<Value> {
    match val {
        ValueRef::Text(bytes) | ValueRef::Blob(bytes) => serde_json::from_slice(bytes).ok(),
        ValueRef::Integer(i) => Some(Value::from(i)),
        ValueRef::Real(f) => serde_json::Number::from_f64(f).map(Value::Number),
        ValueRef::Null => None,
    }
}

struct ColTimingConfig {
    crt: i64,
    ver: i64,
    decks_json: String,
    rollover_hour: i64,
    current_offset: FixedOffset,
    creation_offset: FixedOffset,
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        [table],
        |_| Ok(()),
    )
    .is_ok()
}

fn read_col_timing_config(conn: &Connection) -> rusqlite::Result<Option<ColTimingConfig>> {
    if !table_exists(conn, "col") {
        return Ok(None);
    }

    let mut stmt = conn.prepare("SELECT crt, ver, conf, decks FROM col LIMIT 1")?;
    let mut rows = stmt.query([])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };

    let crt: i64 = row.get(0)?;
    let ver: i64 = row.get(1)?;
    let conf_json: String = row.get(2)?;
    let decks_json: String = row.get(3)?;

    let mut rollover_opt: Option<i64> = None;
    let mut local_offset_opt: Option<i32> = None;
    let mut creation_offset_opt: Option<i32> = None;

    if table_exists(conn, "config") {
        let mut cfg_stmt = conn.prepare(
            "SELECT key, val FROM config WHERE key IN ('rollover', 'localOffset', 'creationOffset')",
        )?;
        let mut cfg_rows = cfg_stmt.query([])?;
        while let Some(cfg_row) = cfg_rows.next()? {
            let key: String = cfg_row.get(0)?;
            let parsed = parse_config_json_value(cfg_row.get_ref(1)?);
            match key.as_str() {
                "rollover" => rollover_opt = parsed.and_then(|v| v.as_i64()),
                "localOffset" => {
                    local_offset_opt = parsed.and_then(|v| v.as_i64()).map(|n| n as i32)
                }
                "creationOffset" => {
                    creation_offset_opt = parsed.and_then(|v| v.as_i64()).map(|n| n as i32)
                }
                _ => {}
            }
        }
    }

    if (rollover_opt.is_none() || local_offset_opt.is_none() || creation_offset_opt.is_none())
        && !conf_json.trim().is_empty()
    {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&conf_json) {
            if rollover_opt.is_none() {
                rollover_opt = map.get("rollover").and_then(|v| v.as_i64());
            }
            if local_offset_opt.is_none() {
                local_offset_opt = map
                    .get("localOffset")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32);
            }
            if creation_offset_opt.is_none() {
                creation_offset_opt = map
                    .get("creationOffset")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32);
            }
        }
    }

    let rollover_hour = rollover_opt.unwrap_or(DEFAULT_ROLLOVER_HOUR).clamp(0, 23);
    let current_offset = fixed_offset_from_minutes_west(local_offset_opt.unwrap_or(0));
    let creation_offset =
        fixed_offset_from_minutes_west(creation_offset_opt.or(local_offset_opt).unwrap_or(0));

    Ok(Some(ColTimingConfig {
        crt,
        ver,
        decks_json,
        rollover_hour,
        current_offset,
        creation_offset,
    }))
}

fn resolve_deck_and_children(
    conn: &Connection,
    cfg: &ColTimingConfig,
    deck_id: i64,
) -> rusqlite::Result<Vec<i64>> {
    if cfg.ver >= 15 && table_exists(conn, "decks") {
        let mut stmt = conn.prepare("SELECT id, name FROM decks")?;
        let mut rows = stmt.query([])?;
        let mut all_decks: Vec<(i64, String)> = Vec::new();
        let mut target_name: Option<String> = None;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            if id == deck_id {
                target_name = Some(name.clone());
            }
            all_decks.push((id, name));
        }
        let Some(name) = target_name else {
            return Ok(vec![deck_id]);
        };
        let prefix_sep = format!("{name}\x1f");
        let prefix_colon = format!("{name}::");
        let ids: Vec<i64> = all_decks
            .into_iter()
            .filter(|(id, n)| {
                *id == deck_id || n.starts_with(&prefix_sep) || n.starts_with(&prefix_colon)
            })
            .map(|(id, _)| id)
            .collect();
        return Ok(if ids.is_empty() { vec![deck_id] } else { ids });
    }

    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&cfg.decks_json) {
        let key = deck_id.to_string();
        if let Some(root_name) = map
            .get(&key)
            .and_then(|d| d.get("name"))
            .and_then(|n| n.as_str())
        {
            let prefix_colon = format!("{root_name}::");
            let prefix_sep = format!("{root_name}\x1f");
            let mut out = Vec::new();
            for (k, val) in &map {
                let Some(name) = val.get("name").and_then(|n| n.as_str()) else {
                    continue;
                };
                if name == root_name
                    || name.starts_with(&prefix_colon)
                    || name.starts_with(&prefix_sep)
                {
                    if let Ok(id) = k.parse::<i64>() {
                        out.push(id);
                    }
                }
            }
            if !out.is_empty() {
                return Ok(out);
            }
        }
    }

    Ok(vec![deck_id])
}

fn summarize_history(
    date_counts: &BTreeMap<NaiveDate, u32>,
    today_date: NaiveDate,
) -> (u32, f64, f64, u32, u32) {
    let today_count = *date_counts.get(&today_date).unwrap_or(&0);
    if date_counts.is_empty() {
        return (0, 0.0, 0.0, 0, 0);
    }

    let first_date = *date_counts.keys().next().unwrap();
    let last_date = (*date_counts.keys().next_back().unwrap()).max(today_date);
    let period_days = (last_date.signed_duration_since(first_date).num_days() + 1).max(1) as f64;

    let total_reviews: u64 = date_counts.values().map(|&c| c as u64).sum();
    let active_days = date_counts.len() as f64;

    let daily_average = ((total_reviews as f64 / period_days) * 10.0).round() / 10.0;
    let days_learned_percent = ((active_days / period_days) * 1000.0).round() / 10.0;

    let mut longest_streak = 0u32;
    let mut current_run = 0u32;
    let mut prev_date: Option<NaiveDate> = None;
    for &d in date_counts.keys() {
        if let Some(prev) = prev_date {
            if d.signed_duration_since(prev).num_days() == 1 {
                current_run += 1;
            } else {
                current_run = 1;
            }
        } else {
            current_run = 1;
        }
        longest_streak = longest_streak.max(current_run);
        prev_date = Some(d);
    }

    // Current streak: count backwards from today if reviewed today, or from yesterday if
    // today hasn't been reviewed yet (matches Glutanimate Review Heatmap behavior).
    let mut current_streak = 0u32;
    let mut cursor = if date_counts.contains_key(&today_date) {
        today_date
    } else {
        today_date - Duration::days(1)
    };
    while date_counts.contains_key(&cursor) {
        current_streak += 1;
        cursor -= Duration::days(1);
    }

    (
        today_count,
        daily_average,
        days_learned_percent,
        current_streak,
        longest_streak,
    )
}

pub fn compute_heatmap_stats(
    conn: &Connection,
    deck_id: Option<i64>,
    now_secs: i64,
) -> rusqlite::Result<HeatmapResponse> {
    let Some(cfg) = read_col_timing_config(conn)? else {
        let fallback_today = anki_study_date(
            now_secs,
            fixed_offset_from_minutes_west(0),
            DEFAULT_ROLLOVER_HOUR,
        )
        .format("%Y-%m-%d")
        .to_string();
        return Ok(empty_heatmap_stats(fallback_today));
    };

    let today_date = anki_study_date(now_secs, cfg.current_offset, cfg.rollover_hour);
    let today_str = today_date.format("%Y-%m-%d").to_string();
    let today_day_index = compute_today_day_index(
        cfg.crt,
        cfg.creation_offset,
        now_secs,
        cfg.current_offset,
        cfg.rollover_hour,
    );

    let deck_ids = match deck_id {
        Some(did) => Some(resolve_deck_and_children(conn, &cfg, did)?),
        None => None,
    };

    let mut date_counts: BTreeMap<NaiveDate, u32> = BTreeMap::new();
    if table_exists(conn, "revlog") {
        match &deck_ids {
            None => {
                let mut stmt = conn.prepare("SELECT id FROM revlog WHERE type < 4")?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let id_ms: i64 = row.get(0)?;
                    let ts_secs = id_ms.div_euclid(1000);
                    let d = anki_study_date(ts_secs, cfg.current_offset, cfg.rollover_hour);
                    *date_counts.entry(d).or_insert(0) += 1;
                }
            }
            Some(ids) => {
                let placeholders = vec!["?"; ids.len()].join(", ");
                let sql = format!(
                    "SELECT r.id FROM revlog r \
                     JOIN cards c ON r.cid = c.id \
                     WHERE r.type < 4 AND (c.did IN ({placeholders}) OR c.odid IN ({placeholders}))"
                );
                let mut params: Vec<rusqlite::types::Value> =
                    Vec::with_capacity(ids.len() * 2);
                for &id in ids {
                    params.push(rusqlite::types::Value::Integer(id));
                }
                for &id in ids {
                    params.push(rusqlite::types::Value::Integer(id));
                }
                let mut stmt = conn.prepare(&sql)?;
                let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
                while let Some(row) = rows.next()? {
                    let id_ms: i64 = row.get(0)?;
                    let ts_secs = id_ms.div_euclid(1000);
                    let d = anki_study_date(ts_secs, cfg.current_offset, cfg.rollover_hour);
                    *date_counts.entry(d).or_insert(0) += 1;
                }
            }
        }
    }

    let mut forecast: BTreeMap<String, u32> = BTreeMap::new();
    if table_exists(conn, "cards") {
        let mut record_due = |due: i64, odue: i64, odid: i64| {
            let effective_due = if odid != 0 { odue } else { due };
            let (offset_days, due_date) = if effective_due > 1_000_000_000 {
                let d = anki_study_date(effective_due, cfg.current_offset, cfg.rollover_hour);
                (d.signed_duration_since(today_date).num_days(), d)
            } else {
                let offset = effective_due - today_day_index;
                (offset, today_date + Duration::days(offset))
            };
            if (0..=FORECAST_HORIZON_DAYS).contains(&offset_days) {
                let key = due_date.format("%Y-%m-%d").to_string();
                *forecast.entry(key).or_insert(0) += 1;
            }
        };

        match &deck_ids {
            None => {
                let mut stmt = conn
                    .prepare("SELECT due, odue, odid FROM cards WHERE queue IN (2, 3)")?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    record_due(row.get(0)?, row.get(1)?, row.get(2)?);
                }
            }
            Some(ids) => {
                let placeholders = vec!["?"; ids.len()].join(", ");
                let sql = format!(
                    "SELECT due, odue, odid FROM cards \
                     WHERE queue IN (2, 3) AND did IN ({placeholders})"
                );
                let params = ids
                    .iter()
                    .map(|&id| rusqlite::types::Value::Integer(id))
                    .collect::<Vec<_>>();
                let mut stmt = conn.prepare(&sql)?;
                let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
                while let Some(row) = rows.next()? {
                    record_due(row.get(0)?, row.get(1)?, row.get(2)?);
                }
            }
        }
    }

    let (today_count, daily_average, days_learned_percent, current_streak, longest_streak) =
        summarize_history(&date_counts, today_date);

    let history = date_counts
        .into_iter()
        .map(|(d, count)| (d.format("%Y-%m-%d").to_string(), count))
        .collect();

    Ok(HeatmapResponse {
        history,
        forecast,
        today: today_str,
        today_count,
        daily_average,
        days_learned_percent,
        current_streak,
        longest_streak,
    })
}

pub async fn get_heatmap_stats_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HeatmapQuery>,
) -> Response {
    let Some(claims) = authenticate_request(&state, &headers) else {
        return json_err(StatusCode::UNAUTHORIZED, "Authentication required");
    };

    let runtime = match state.users.get_or_init(&claims.u) {
        Ok(rt) => rt,
        Err(e) => return json_err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let now_secs = now_unix_secs();
    if !runtime.col_path.exists() {
        let today = anki_study_date(
            now_secs,
            fixed_offset_from_minutes_west(0),
            DEFAULT_ROLLOVER_HOUR,
        )
        .format("%Y-%m-%d")
        .to_string();
        return Json(empty_heatmap_stats(today)).into_response();
    }

    let col_path = runtime.col_path.clone();
    let deck_id = query.deck_id;
    let res = runtime
        .with_closed_collection(move |_| {
            let conn = Connection::open_with_flags(&col_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
            let _ = conn.create_collation("unicase", |a, b| {
                a.to_lowercase().cmp(&b.to_lowercase())
            });
            Ok(compute_heatmap_stats(&conn, deck_id, now_secs)?)
        })
        .await;

    match res {
        Ok(stats) => Json(stats).into_response(),
        Err(err) => json_err(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to compute heatmap stats: {err}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_test_schema(conn: &Connection, crt: i64, rollover: i64, local_offset_mins: i32) {
        conn.execute_batch(
            r#"
            CREATE TABLE col (
                id integer PRIMARY KEY,
                crt integer NOT NULL,
                mod integer NOT NULL,
                scm integer NOT NULL,
                ver integer NOT NULL,
                dty integer NOT NULL,
                usn integer NOT NULL,
                ls integer NOT NULL,
                conf text NOT NULL,
                models text NOT NULL,
                decks text NOT NULL,
                dconf text NOT NULL,
                tags text NOT NULL
            );
            CREATE TABLE config (
                key text NOT NULL PRIMARY KEY,
                usn integer NOT NULL,
                mtime_secs integer NOT NULL,
                val blob NOT NULL
            );
            CREATE TABLE decks (
                id integer PRIMARY KEY NOT NULL,
                name text NOT NULL,
                mtime_secs integer NOT NULL,
                usn integer NOT NULL,
                common blob NOT NULL,
                kind blob NOT NULL
            );
            CREATE TABLE cards (
                id integer PRIMARY KEY,
                nid integer NOT NULL,
                did integer NOT NULL,
                ord integer NOT NULL,
                mod integer NOT NULL,
                usn integer NOT NULL,
                type integer NOT NULL,
                queue integer NOT NULL,
                due integer NOT NULL,
                ivl integer NOT NULL,
                factor integer NOT NULL,
                reps integer NOT NULL,
                lapses integer NOT NULL,
                left integer NOT NULL,
                odue integer NOT NULL,
                odid integer NOT NULL,
                flags integer NOT NULL,
                data text NOT NULL
            );
            CREATE TABLE revlog (
                id integer PRIMARY KEY,
                cid integer NOT NULL,
                usn integer NOT NULL,
                ease integer NOT NULL,
                ivl integer NOT NULL,
                lastIvl integer NOT NULL,
                factor integer NOT NULL,
                time integer NOT NULL,
                type integer NOT NULL
            );
            "#,
        )
        .unwrap();

        conn.execute(
            "INSERT INTO col VALUES (1, ?1, 0, 0, 18, 0, 0, 0, '{}', '{}', '{}', '{}', '{}')",
            [crt],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO config VALUES ('rollover', 0, 0, ?1)",
            [rollover.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO config VALUES ('localOffset', 0, 0, ?1)",
            [local_offset_mins.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO config VALUES ('creationOffset', 0, 0, ?1)",
            [local_offset_mins.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO decks VALUES (1, 'Default', 0, 0, X'', X'')",
            [],
        )
        .unwrap();
    }

    fn ts_utc(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> i64 {
        chrono::Utc
            .with_ymd_and_hms(y, m, d, hh, mm, 0)
            .single()
            .unwrap()
            .timestamp()
    }

    #[test]
    fn empty_collection_returns_zero_stats() {
        let conn = Connection::open_in_memory().unwrap();
        let crt = ts_utc(2026, 1, 1, 4, 0);
        create_test_schema(&conn, crt, 4, 0);

        let now = ts_utc(2026, 11, 26, 12, 0);
        let res = compute_heatmap_stats(&conn, None, now).unwrap();

        assert!(res.history.is_empty());
        assert!(res.forecast.is_empty());
        assert_eq!(res.today, "2026-11-26");
        assert_eq!(res.today_count, 0);
        assert_eq!(res.daily_average, 0.0);
        assert_eq!(res.days_learned_percent, 0.0);
        assert_eq!(res.current_streak, 0);
        assert_eq!(res.longest_streak, 0);
    }

    #[test]
    fn computes_history_rollover_and_streaks() {
        let conn = Connection::open_in_memory().unwrap();
        let crt = ts_utc(2026, 11, 1, 4, 0);
        create_test_schema(&conn, crt, 4, 0);

        conn.execute(
            "INSERT INTO cards VALUES (10, 1, 1, 0, 0, 0, 2, 2, 25, 5, 2500, 3, 0, 0, 0, 0, 0, '')",
            [],
        )
        .unwrap();

        // Longest streak: Nov 10, 11, 12, 13 (4 days)
        // Gap on Nov 14..23
        // Current streak: Nov 24, 25, 26 (3 days), including a 02:30 UTC review on Nov 26
        // which belongs to Nov 25 under rollover=4!
        let review_timestamps_ms = [
            ts_utc(2026, 11, 10, 10, 0) * 1000,
            ts_utc(2026, 11, 11, 10, 0) * 1000,
            ts_utc(2026, 11, 12, 10, 0) * 1000,
            ts_utc(2026, 11, 13, 10, 0) * 1000,
            ts_utc(2026, 11, 24, 15, 0) * 1000,
            ts_utc(2026, 11, 25, 18, 0) * 1000,
            ts_utc(2026, 11, 26, 2, 30) * 1000, // before 4am -> Nov 25!
            ts_utc(2026, 11, 26, 11, 0) * 1000, // after 4am -> Nov 26!
            ts_utc(2026, 11, 26, 11, 5) * 1000, // Nov 26!
        ];
        for (idx, ts) in review_timestamps_ms.iter().enumerate() {
            conn.execute(
                "INSERT INTO revlog VALUES (?1, 10, 0, 3, 5, 3, 2500, 1200, 1)",
                [ts + idx as i64],
            )
            .unwrap();
        }
        // Manual/rescheduled revlog (type=4) should be ignored
        conn.execute(
            "INSERT INTO revlog VALUES (?1, 10, 0, 0, 5, 3, 0, 0, 4)",
            [ts_utc(2026, 11, 20, 12, 0) * 1000],
        )
        .unwrap();

        let now = ts_utc(2026, 11, 26, 15, 0);
        let res = compute_heatmap_stats(&conn, None, now).unwrap();

        assert_eq!(res.today, "2026-11-26");
        assert_eq!(res.today_count, 2);
        assert_eq!(res.history.get("2026-11-25"), Some(&2));
        assert_eq!(res.history.get("2026-11-26"), Some(&2));
        assert_eq!(res.history.get("2026-11-20"), None);
        assert_eq!(res.longest_streak, 4);
        assert_eq!(res.current_streak, 3);
        // 7 active days out of 17 days (Nov 10 .. Nov 26 inclusive)
        assert_eq!(res.days_learned_percent, 41.2);
    }

    #[test]
    fn current_streak_survives_before_first_review_today() {
        let conn = Connection::open_in_memory().unwrap();
        let crt = ts_utc(2026, 11, 1, 4, 0);
        create_test_schema(&conn, crt, 4, 0);

        for (idx, day) in [23, 24, 25].iter().enumerate() {
            conn.execute(
                "INSERT INTO revlog VALUES (?1, 10, 0, 3, 1, 0, 2500, 1000, 0)",
                [ts_utc(2026, 11, *day, 14, 0) * 1000 + idx as i64],
            )
            .unwrap();
        }

        let now = ts_utc(2026, 11, 26, 10, 0);
        let res = compute_heatmap_stats(&conn, None, now).unwrap();
        assert_eq!(res.today_count, 0);
        assert_eq!(res.current_streak, 3);
        assert_eq!(res.longest_streak, 3);
    }

    #[test]
    fn computes_forecast_and_deck_filtering() {
        let conn = Connection::open_in_memory().unwrap();
        let crt = ts_utc(2026, 11, 1, 4, 0);
        create_test_schema(&conn, crt, 4, 0);

        // Parent deck 10, child deck 11, unrelated deck 20
        conn.execute(
            "INSERT INTO decks VALUES (10, 'Lang', 0, 0, X'', X''), (11, 'Lang\x1fJapanese', 0, 0, X'', X''), (20, 'Science', 0, 0, X'', X'')",
            [],
        )
        .unwrap();

        // Today is Nov 26 => days_elapsed since Nov 1 = 25
        let now = ts_utc(2026, 11, 26, 12, 0);

        // Card 1 in deck 10 due today (day 25)
        conn.execute(
            "INSERT INTO cards VALUES (1, 1, 10, 0, 0, 0, 2, 2, 25, 1, 2500, 1, 0, 0, 0, 0, 0, '')",
            [],
        ).unwrap();
        // Card 2 in child deck 11 due in +2 days (day 27 -> 2026-11-28)
        conn.execute(
            "INSERT INTO cards VALUES (2, 2, 11, 0, 0, 0, 2, 2, 27, 3, 2500, 1, 0, 0, 0, 0, 0, '')",
            [],
        ).unwrap();
        // Card 3 in deck 11 day-learning (queue 3) due via timestamp on 2026-11-28
        conn.execute(
            "INSERT INTO cards VALUES (3, 3, 11, 0, 0, 0, 1, 3, ?1, 1, 2500, 1, 0, 0, 0, 0, 0, '')",
            [ts_utc(2026, 11, 28, 8, 0)],
        ).unwrap();
        // Card 4 in deck 20 due in +2 days (should be excluded when filtering deck_id=10)
        conn.execute(
            "INSERT INTO cards VALUES (4, 4, 20, 0, 0, 0, 2, 2, 27, 3, 2500, 1, 0, 0, 0, 0, 0, '')",
            [],
        ).unwrap();
        // Card 5 overdue (day 20) & Card 6 beyond 90 days (day 200) -> excluded from 90-day forecast
        conn.execute(
            "INSERT INTO cards VALUES (5, 5, 10, 0, 0, 0, 2, 2, 20, 3, 2500, 1, 0, 0, 0, 0, 0, '')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO cards VALUES (6, 6, 10, 0, 0, 0, 2, 2, 200, 3, 2500, 1, 0, 0, 0, 0, 0, '')",
            [],
        ).unwrap();

        let all = compute_heatmap_stats(&conn, None, now).unwrap();
        assert_eq!(all.forecast.get("2026-11-26"), Some(&1));
        assert_eq!(all.forecast.get("2026-11-28"), Some(&3));
        assert_eq!(all.forecast.len(), 2);

        let lang_only = compute_heatmap_stats(&conn, Some(10), now).unwrap();
        assert_eq!(lang_only.forecast.get("2026-11-26"), Some(&1));
        assert_eq!(lang_only.forecast.get("2026-11-28"), Some(&2));
    }
}
