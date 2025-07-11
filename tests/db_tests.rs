use workhours::db::{Database, Holiday};
use tempfile::tempdir;

#[test]
fn test_database_operations() {
    // Create a temporary directory for the test database
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
    
    // Create a new database
    let db = Database::new(&db_path).unwrap();
    
    // Test adding a holiday
    let holiday = Holiday {
        id: None,
        date: "2023-12-25T00:00:00Z".to_string(),
        description: "Christmas".to_string(),
        country: "us".to_string(),
    };
    
    let id = db.add_holiday(&holiday).unwrap();
    assert!(id > 0);
    
    // Test getting holidays by country
    let holidays = db.get_holidays_by_country("us").unwrap();
    assert_eq!(holidays.len(), 1);
    assert_eq!(holidays[0].date, "2023-12-25T00:00:00Z");
    assert_eq!(holidays[0].description, "Christmas");
    assert_eq!(holidays[0].country, "us");
    
    // Test getting all holidays
    let all_holidays = db.get_all_holidays().unwrap();
    assert_eq!(all_holidays.len(), 1);
    
    // Test adding another holiday for a different country
    let holiday2 = Holiday {
        id: None,
        date: "2023-07-14T00:00:00Z".to_string(),
        description: "Bastille Day".to_string(),
        country: "fr".to_string(),
    };
    
    let id2 = db.add_holiday(&holiday2).unwrap();
    assert!(id2 > 0);
    
    // Test getting holidays by country again
    let fr_holidays = db.get_holidays_by_country("fr").unwrap();
    assert_eq!(fr_holidays.len(), 1);
    assert_eq!(fr_holidays[0].date, "2023-07-14T00:00:00Z");
    
    // Test getting all holidays again
    let all_holidays = db.get_all_holidays().unwrap();
    assert_eq!(all_holidays.len(), 2);
    
    // Test deleting a holiday
    db.delete_holiday(id).unwrap();
    let us_holidays = db.get_holidays_by_country("us").unwrap();
    assert_eq!(us_holidays.len(), 0);
    
    // Test getting all holidays after deletion
    let all_holidays = db.get_all_holidays().unwrap();
    assert_eq!(all_holidays.len(), 1);
}

#[test]
fn test_database_error_handling() {
    // Test with an invalid database path
    let result = Database::new("/invalid/path/to/db.sqlite");
    assert!(result.is_err());
    
    // Create a valid database
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
    let db = Database::new(&db_path).unwrap();
    
    // Test getting holidays for a non-existent country
    let holidays = db.get_holidays_by_country("nonexistent").unwrap();
    assert_eq!(holidays.len(), 0);
}

#[test]
fn test_database_clone() {
    // Test that the database can be cloned
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db").to_str().unwrap().to_string();
    
    let db1 = Database::new(&db_path).unwrap();
    
    // Add a holiday to the first database
    let holiday = Holiday {
        id: None,
        date: "2023-12-25T00:00:00Z".to_string(),
        description: "Christmas".to_string(),
        country: "us".to_string(),
    };
    
    db1.add_holiday(&holiday).unwrap();
    
    // Clone the database
    let db2 = db1.clone();
    
    // Check that the holiday exists in the cloned database
    let holidays = db2.get_holidays_by_country("us").unwrap();
    assert_eq!(holidays.len(), 1);
    assert_eq!(holidays[0].date, "2023-12-25T00:00:00Z");
}