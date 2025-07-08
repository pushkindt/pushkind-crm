mod common;

#[test]
fn test_creates_and_removes_db_files() {
    let test_db = common::TestDb::new("test_in_memory_connection.db");
    let conn = test_db.pool().get();
    assert!(conn.is_ok());
}
