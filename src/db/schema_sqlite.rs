diesel::table! {
    room_mappings (id) {
        id -> BigInt,
        matrix_room_id -> Text,
        dingtalk_chat_id -> Text,
        dingtalk_chat_name -> Text,
        dingtalk_conversation_type -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    user_mappings (id) {
        id -> BigInt,
        matrix_user_id -> Text,
        dingtalk_user_id -> Text,
        dingtalk_username -> Text,
        dingtalk_nick -> Nullable<Text>,
        dingtalk_avatar -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    processed_events (id) {
        id -> BigInt,
        event_id -> Text,
        event_type -> Text,
        source -> Text,
        processed_at -> Timestamp,
    }
}

diesel::table! {
    message_mappings (id) {
        id -> BigInt,
        dingtalk_message_id -> Text,
        matrix_room_id -> Text,
        matrix_event_id -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    room_mappings,
    user_mappings,
    processed_events,
    message_mappings,
);
