use rusqlite::{params, Connection};
use std::path::Path;

#[allow(dead_code)]
pub struct HistoryRecord {
    pub id: i64,
    pub task_name: String,
    pub task_id: String,
    pub code: String,
    pub created_at: String,
}

fn ensure_db(path: &str) -> Result<Connection, String> {
    let db_path = Path::new(path);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建存储目录失败: {e}"))?;
    }

    let conn = Connection::open(path)
        .map_err(|e| format!("打开数据库失败: {e}"))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS push_history (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            task_name   TEXT NOT NULL,
            task_id     TEXT NOT NULL,
            code        TEXT NOT NULL,
            created_at  TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )
    .map_err(|e| format!("创建数据表失败: {e}"))?;

    Ok(conn)
}

pub fn record_push(path: &str, task_name: &str, task_id: &str, code: &str) -> Result<HistoryRecord, String> {
    let conn = ensure_db(path)?;

    conn.execute(
        "INSERT INTO push_history (task_name, task_id, code) VALUES (?1, ?2, ?3)",
        params![task_name, task_id, code],
    )
    .map_err(|e| format!("插入记录失败: {e}"))?;

    let id = conn.last_insert_rowid();

    let mut stmt = conn
        .prepare("SELECT id, task_name, task_id, code, created_at FROM push_history WHERE id = ?1")
        .map_err(|e| format!("查询记录失败: {e}"))?;

    let record = stmt
        .query_row(params![id], |row| {
            Ok(HistoryRecord {
                id: row.get(0)?,
                task_name: row.get(1)?,
                task_id: row.get(2)?,
                code: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| format!("读取记录失败: {e}"))?;

    Ok(record)
}

#[allow(dead_code)]
pub fn list_history(path: &str, limit: usize) -> Result<Vec<HistoryRecord>, String> {
    let conn = ensure_db(path)?;

    let mut stmt = conn
        .prepare("SELECT id, task_name, task_id, code, created_at FROM push_history ORDER BY id DESC LIMIT ?1")
        .map_err(|e| format!("准备查询失败: {e}"))?;

    let records = stmt
        .query_map(params![limit as i64], |row| {
            Ok(HistoryRecord {
                id: row.get(0)?,
                task_name: row.get(1)?,
                task_id: row.get(2)?,
                code: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| format!("查询历史记录失败: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(records)
}
