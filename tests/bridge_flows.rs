use chrono::Utc;
use matrix_bridge_dingtalk::bridge::DingTalkBridge;
use matrix_bridge_dingtalk::config::Config;
use matrix_bridge_dingtalk::database::{Database, DeadLetterEvent};
use tempfile::NamedTempFile;

fn load_test_config(db_url: &str) -> Config {
    let mut config: Config =
        serde_yaml::from_str(include_str!("../config/config.sample.yaml"))
            .expect("sample config should parse");
    config.database.url = Some(db_url.to_string());
    config.bridge.domain = "localhost".to_string();
    config.bridge.homeserver_url = "http://localhost:8008".to_string();
    config.registration.bridge_id = "dingtalk-test".to_string();
    config.registration.appservice_token = "as_token".to_string();
    config.registration.homeserver_token = "hs_token".to_string();
    config.registration.sender_localpart = "_dingtalk_bot".to_string();
    config
}

fn make_dead_letter(status: &str, dedupe_key: &str) -> DeadLetterEvent {
    let now = Utc::now();
    DeadLetterEvent {
        id: 0,
        source: "matrix".to_string(),
        event_type: "m.room.message".to_string(),
        dedupe_key: dedupe_key.to_string(),
        conversation_id: Some("conv-test".to_string()),
        payload: r#"{"content":"hello"}"#.to_string(),
        error: "boom".to_string(),
        status: status.to_string(),
        replay_count: 0,
        last_replayed_at: None,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn dead_letter_cleanup_uses_bound_parameters() {
    let temp = NamedTempFile::new().expect("create temp file");
    let db_url = format!("sqlite://{}", temp.path().display());

    let db = Database::connect("sqlite", &db_url, 4, 1)
        .await
        .expect("connect db");
    db.run_migrations().await.expect("run migrations");
    let dead_letter_store = db.stores().dead_letter_store();

    dead_letter_store
        .insert_dead_letter(&make_dead_letter("pending", "dedupe-a"))
        .await
        .expect("insert dead-letter A");
    dead_letter_store
        .insert_dead_letter(&make_dead_letter("failed", "dedupe-b"))
        .await
        .expect("insert dead-letter B");

    let deleted = dead_letter_store
        .cleanup_dead_letters(Some("pending' OR 1=1 --"), None, 100)
        .await
        .expect("cleanup should succeed");
    assert_eq!(deleted, 0);

    let total = dead_letter_store
        .count_dead_letters(None)
        .await
        .expect("count total");
    assert_eq!(total, 2);

    let deleted_pending = dead_letter_store
        .cleanup_dead_letters(Some("pending"), None, 100)
        .await
        .expect("cleanup pending should succeed");
    assert_eq!(deleted_pending, 1);
}

#[tokio::test]
async fn bridge_room_mapping_crud_roundtrip() {
    let temp = NamedTempFile::new().expect("create temp file");
    let db_url = format!("sqlite://{}", temp.path().display());

    let db = Database::connect("sqlite", &db_url, 4, 1)
        .await
        .expect("connect db");
    db.run_migrations().await.expect("run migrations");

    let config = load_test_config(&db_url);
    let bridge = DingTalkBridge::new(config, db)
        .await
        .expect("init bridge");

    let mapping = bridge
        .bridge_room("!room:test.local", "conv-test", Some("Bridge Test".to_string()))
        .await
        .expect("bridge room");
    assert_eq!(mapping.matrix_room_id, "!room:test.local");
    assert_eq!(mapping.dingtalk_conversation_id, "conv-test");

    let mappings = bridge
        .list_room_mappings(10, 0)
        .await
        .expect("list mappings");
    assert_eq!(mappings.len(), 1);

    let removed = bridge
        .unbridge_room("!room:test.local")
        .await
        .expect("unbridge room");
    assert!(removed);

    let mappings_after = bridge
        .list_room_mappings(10, 0)
        .await
        .expect("list mappings after delete");
    assert!(mappings_after.is_empty());
}
