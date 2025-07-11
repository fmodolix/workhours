use rusqlite::{Connection, Result, params};
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Holiday {
    pub id: Option<i32>,
    pub date: String,
    pub description: String,
    pub country: String,
}

pub struct Database {
    conn: Mutex<Connection>,
    path: String,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS holidays (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                description TEXT NOT NULL,
                country TEXT NOT NULL
            )",
            [],
        )?;
        Ok(Database { 
            conn: Mutex::new(conn),
            path: path.to_string(),
        })
    }

    pub fn add_holiday(&self, holiday: &Holiday) -> Result<i32> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO holidays (date, description, country) VALUES (?, ?, ?)",
            params![holiday.date, holiday.description, holiday.country],
        )?;
        Ok(conn.last_insert_rowid() as i32)
    }

    pub fn get_holidays_by_country(&self, country: &str) -> Result<Vec<Holiday>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM holidays WHERE country = ?")?;
        let holidays = stmt.query_map(params![country], |row| {
            Ok(Holiday {
                id: Some(row.get(0)?),
                date: row.get(1)?,
                description: row.get(2)?,
                country: row.get(3)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(holidays)
    }

    pub fn get_all_holidays(&self) -> Result<Vec<Holiday>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM holidays")?;
        let holidays = stmt.query_map([], |row| {
            Ok(Holiday {
                id: Some(row.get(0)?),
                date: row.get(1)?,
                description: row.get(2)?,
                country: row.get(3)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        Ok(holidays)
    }

    pub fn delete_holiday(&self, id: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM holidays WHERE id = ?", params![id])?;
        Ok(())
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        // Create a new database with the same path
        Database::new(&self.path).expect("Failed to clone database")
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_holiday_operations() {
        let dir = tempdir().unwrap();
        let db = Database::new(dir.path().join("test.db").to_str().unwrap()).unwrap();

        let holiday = Holiday {
            id: None,
            date: "2025-07-04".to_string(),
            description: "Independence Day".to_string(),
            country: "US".to_string(),
        };

        let id = db.add_holiday(&holiday).unwrap();
        let holidays = db.get_holidays_by_country("US").unwrap();
        assert_eq!(holidays.len(), 1);
        assert_eq!(holidays[0].id, Some(id));

        db.delete_holiday(id).unwrap();
        let holidays = db.get_holidays_by_country("US").unwrap();
        assert_eq!(holidays.len(), 0);
    }
}
