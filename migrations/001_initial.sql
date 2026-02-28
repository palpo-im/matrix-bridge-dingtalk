-- 用户映射表
CREATE TABLE IF NOT EXISTS user_mappings (
    id BIGSERIAL PRIMARY KEY,
    matrix_user_id TEXT NOT NULL UNIQUE,
    dingtalk_user_id TEXT NOT NULL UNIQUE,
    dingtalk_username TEXT NOT NULL,
    dingtalk_nick TEXT,
    dingtalk_avatar TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 房间映射表
CREATE TABLE IF NOT EXISTS room_mappings (
    id BIGSERIAL PRIMARY KEY,
    matrix_room_id TEXT NOT NULL UNIQUE,
    dingtalk_chat_id TEXT NOT NULL UNIQUE,
    dingtalk_chat_name TEXT NOT NULL,
    dingtalk_conversation_type TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 事件跟踪表
CREATE TABLE IF NOT EXISTS processed_events (
    id BIGSERIAL PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE,
    event_type TEXT NOT NULL,
    source TEXT NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 消息映射表
CREATE TABLE IF NOT EXISTS message_mappings (
    id BIGSERIAL PRIMARY KEY,
    dingtalk_message_id TEXT NOT NULL,
    matrix_room_id TEXT NOT NULL,
    matrix_event_id TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(dingtalk_message_id, matrix_room_id)
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_user_mappings_matrix_id ON user_mappings(matrix_user_id);
CREATE INDEX IF NOT EXISTS idx_user_mappings_dingtalk_id ON user_mappings(dingtalk_user_id);
CREATE INDEX IF NOT EXISTS idx_room_mappings_matrix_id ON room_mappings(matrix_room_id);
CREATE INDEX IF NOT EXISTS idx_room_mappings_dingtalk_id ON room_mappings(dingtalk_chat_id);
CREATE INDEX IF NOT EXISTS idx_processed_events_event_id ON processed_events(event_id);
CREATE INDEX IF NOT EXISTS idx_message_mappings_matrix_event ON message_mappings(matrix_event_id);
CREATE INDEX IF NOT EXISTS idx_message_mappings_dingtalk_msg ON message_mappings(dingtalk_message_id);
