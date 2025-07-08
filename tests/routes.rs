use actix_web_flash_messages::Level;
use pushkind_crm::routes::alert_level_to_str;

#[test]
fn test_alert_level_to_str_mappings() {
    assert_eq!(alert_level_to_str(&Level::Error), "danger");
    assert_eq!(alert_level_to_str(&Level::Warning), "warning");
    assert_eq!(alert_level_to_str(&Level::Success), "success");
    assert_eq!(alert_level_to_str(&Level::Info), "info");
    assert_eq!(alert_level_to_str(&Level::Debug), "info");
}
