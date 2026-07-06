//! SQLite storage for clipboard history + pinned items (backend-owned).
//!
//! One table with a `pinned` flag: a pinned item is just a history row that
//! survives the cap and sorts to the top. Ordering is by recency of last use
//! (`last_used_at`), which is bumped when the same content is copied again
//! (Windows-style "move to top on re-copy").

use crate::models::Clip;
use crate::util::now_ms;
use rusqlite::{params, Connection, Row};
use std::path::Path;

const COLS: &str =
    "id, kind, content, html, image_path, thumb_path, width, height, byte_size, pinned, created_at, last_used_at";

/// A row about to be inserted.
pub struct NewClip<'a> {
    pub kind: &'a str,
    pub content: Option<&'a str>,
    /// Rich-text `text/html` for text clips (see `Clip::html`); `None` otherwise.
    pub html: Option<&'a str>,
    pub image_path: Option<&'a str>,
    pub thumb_path: Option<&'a str>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub byte_size: Option<i64>,
    pub hash: &'a str,
}

/// Image paths (full, thumb) of a deleted row, for filesystem GC.
pub type GcPaths = (Option<String>, Option<String>);

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clips (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            kind         TEXT    NOT NULL CHECK (kind IN ('text','image')),
            content      TEXT,
            html         TEXT,
            image_path   TEXT,
            thumb_path   TEXT,
            width        INTEGER,
            height       INTEGER,
            byte_size    INTEGER,
            hash         TEXT    NOT NULL,
            pinned       INTEGER NOT NULL DEFAULT 0,
            created_at   INTEGER NOT NULL,
            last_used_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_clips_used   ON clips(last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clips_pinned ON clips(pinned, last_used_at DESC);
        CREATE INDEX IF NOT EXISTS idx_clips_hash   ON clips(hash);
        CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);",
    )?;

    // Migration: `html` was added after the initial release. `CREATE TABLE IF
    // NOT EXISTS` above won't alter a pre-existing table, so add the column here.
    // Guarded by a column check so we don't run a failing ALTER on every startup.
    let has_html = conn
        .prepare("SELECT 1 FROM pragma_table_info('clips') WHERE name = 'html'")
        .and_then(|mut s| s.exists([]))
        .unwrap_or(false);
    if !has_html {
        conn.execute("ALTER TABLE clips ADD COLUMN html TEXT", [])?;
    }
    Ok(())
}

fn row_to_clip(row: &Row) -> rusqlite::Result<Clip> {
    Ok(Clip {
        id: row.get("id")?,
        kind: row.get("kind")?,
        content: row.get("content")?,
        html: row.get("html")?,
        image_path: row.get("image_path")?,
        thumb_path: row.get("thumb_path")?,
        width: row.get("width")?,
        height: row.get("height")?,
        byte_size: row.get("byte_size")?,
        pinned: row.get::<_, i64>("pinned")? != 0,
        created_at: row.get("created_at")?,
        last_used_at: row.get("last_used_at")?,
    })
}

fn query_clips<P: rusqlite::Params>(conn: &Connection, sql: &str, p: P) -> Vec<Clip> {
    let mut out = Vec::new();
    if let Ok(mut stmt) = conn.prepare(sql) {
        if let Ok(rows) = stmt.query_map(p, row_to_clip) {
            out.extend(rows.flatten());
        }
    }
    out
}

pub fn insert(conn: &Connection, c: &NewClip) -> rusqlite::Result<i64> {
    let now = now_ms();
    conn.execute(
        "INSERT INTO clips
            (kind, content, html, image_path, thumb_path, width, height, byte_size, hash, pinned, created_at, last_used_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,?10,?10)",
        params![
            c.kind, c.content, c.html, c.image_path, c.thumb_path, c.width, c.height, c.byte_size, c.hash, now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Find the most recent existing row with this content hash, if any.
pub fn find_by_hash(conn: &Connection, hash: &str) -> Option<i64> {
    conn.query_row(
        "SELECT id FROM clips WHERE hash = ?1 ORDER BY last_used_at DESC LIMIT 1",
        params![hash],
        |r| r.get(0),
    )
    .ok()
}

/// Bump an item to the top (used on re-copy of existing content).
pub fn bump_used(conn: &Connection, id: i64) {
    let _ = conn.execute(
        "UPDATE clips SET last_used_at = ?2 WHERE id = ?1",
        params![id, now_ms()],
    );
}

/// Bump on re-copy, refreshing stored HTML when the re-copied content carries
/// formatting. Upgrade-only: a later plain-text re-copy (`html == None`) keeps
/// formatting captured earlier rather than clearing it.
pub fn bump_used_with_html(conn: &Connection, id: i64, html: Option<&str>) {
    match html {
        Some(h) => {
            let _ = conn.execute(
                "UPDATE clips SET last_used_at = ?2, html = ?3 WHERE id = ?1",
                params![id, now_ms(), h],
            );
        }
        None => bump_used(conn, id),
    }
}

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE clips SET pinned = ?2 WHERE id = ?1",
        params![id, pinned as i64],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: i64) -> Option<Clip> {
    query_clips(conn, &format!("SELECT {COLS} FROM clips WHERE id = ?1"), params![id])
        .into_iter()
        .next()
}

pub fn list_history(conn: &Connection, limit: i64) -> Vec<Clip> {
    query_clips(
        conn,
        &format!("SELECT {COLS} FROM clips ORDER BY pinned DESC, last_used_at DESC, id DESC LIMIT ?1"),
        params![limit],
    )
}

pub fn list_pins(conn: &Connection) -> Vec<Clip> {
    query_clips(
        conn,
        &format!("SELECT {COLS} FROM clips WHERE pinned = 1 ORDER BY last_used_at DESC, id DESC"),
        [],
    )
}

pub fn search(conn: &Connection, q: &str, limit: i64) -> Vec<Clip> {
    let escaped = q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let like = format!("%{escaped}%");
    query_clips(
        conn,
        &format!(
            "SELECT {COLS} FROM clips WHERE kind = 'text' AND content LIKE ?1 ESCAPE '\\' \
             ORDER BY pinned DESC, last_used_at DESC, id DESC LIMIT ?2"
        ),
        params![like, limit],
    )
}

/// Delete one row; returns its image paths (if any) for filesystem GC.
pub fn delete(conn: &Connection, id: i64) -> Vec<GcPaths> {
    let paths = conn
        .query_row(
            "SELECT image_path, thumb_path FROM clips WHERE id = ?1",
            params![id],
            |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .ok();
    let _ = conn.execute("DELETE FROM clips WHERE id = ?1", params![id]);
    paths.into_iter().collect()
}

/// Clear history. When `keep_pinned` is true, pinned items are preserved.
pub fn clear(conn: &Connection, keep_pinned: bool) -> Vec<GcPaths> {
    let where_clause = if keep_pinned { "WHERE pinned = 0" } else { "" };
    let mut gc = Vec::new();
    if let Ok(mut stmt) =
        conn.prepare(&format!("SELECT image_path, thumb_path FROM clips {where_clause}"))
    {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?))
        }) {
            gc.extend(rows.flatten());
        }
    }
    let _ = conn.execute(&format!("DELETE FROM clips {where_clause}"), []);
    gc
}

/// Evict non-pinned rows beyond `cap`, keeping the most-recently-used. Returns
/// image paths of evicted rows for filesystem GC.
pub fn enforce_cap(conn: &Connection, cap: u32) -> Vec<GcPaths> {
    let keep_sql =
        "SELECT id FROM clips WHERE pinned = 0 ORDER BY last_used_at DESC, id DESC LIMIT ?1";
    let mut gc = Vec::new();
    if let Ok(mut stmt) = conn.prepare(&format!(
        "SELECT image_path, thumb_path FROM clips WHERE pinned = 0 AND id NOT IN ({keep_sql})"
    )) {
        if let Ok(rows) = stmt.query_map(params![cap], |r| {
            Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?))
        }) {
            gc.extend(rows.flatten());
        }
    }
    let _ = conn.execute(
        &format!("DELETE FROM clips WHERE pinned = 0 AND id NOT IN ({keep_sql})"),
        params![cap],
    );
    gc
}

/// All image/thumb paths referenced by the DB (for startup reconcile).
pub fn referenced_image_paths(conn: &Connection) -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT image_path FROM clips WHERE image_path IS NOT NULL \
         UNION SELECT thumb_path FROM clips WHERE thumb_path IS NOT NULL",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| r.get::<_, Option<String>>(0)) {
            v.extend(rows.flatten().flatten());
        }
    }
    v
}

/// Rows whose backing image file has vanished (returns their ids).
pub fn image_rows(conn: &Connection) -> Vec<(i64, Option<String>)> {
    let mut v = Vec::new();
    if let Ok(mut stmt) =
        conn.prepare("SELECT id, image_path FROM clips WHERE kind = 'image'")
    {
        if let Ok(rows) =
            stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)))
        {
            v.extend(rows.flatten());
        }
    }
    v
}
