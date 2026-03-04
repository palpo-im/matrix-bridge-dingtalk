use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};

use super::ConfigError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub registration: RegistrationConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub room: RoomConfig,
    #[serde(default)]
    pub channel: ChannelConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    #[serde(default)]
    pub ghosts: GhostsConfig,
    #[serde(default)]
    pub stream: StreamConfig,
    #[serde(default)]
    pub callback: CallbackConfig,
    #[serde(default)]
    pub metrics: MetricsConfig,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path =
            std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());

        Self::load_from_path(&config_path)
    }

    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let mut config: Config = serde_yaml::from_str(&content)?;

        config.apply_env_overrides()?;
        config.validate()?;

        Ok(config)
    }

    pub fn load_from_bytes(bytes: &[u8]) -> Result<Self, ConfigError> {
        let mut config: Config = serde_yaml::from_slice(bytes)?;
        config.apply_env_overrides()?;
        config.validate()?;
        Ok(config)
    }

    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(token) = std::env::var("APPSERVICE_DINGTALK_REGISTRATION_AS_TOKEN") {
            self.registration.appservice_token = token;
        }
        if let Ok(token) = std::env::var("APPSERVICE_DINGTALK_REGISTRATION_HS_TOKEN") {
            self.registration.homeserver_token = token;
        }
        if let Ok(id) = std::env::var("APPSERVICE_DINGTALK_REGISTRATION_ID") {
            self.registration.bridge_id = id;
        }
        if let Ok(token) = std::env::var("MATRIX_BRIDGE_DINGTALK_AS_TOKEN") {
            if !token.trim().is_empty() {
                self.registration.appservice_token = token;
            }
        }
        if let Ok(token) = std::env::var("MATRIX_BRIDGE_DINGTALK_HS_TOKEN") {
            if !token.trim().is_empty() {
                self.registration.homeserver_token = token;
            }
        }
        if let Ok(uri) = std::env::var("MATRIX_BRIDGE_DINGTALK_DB_URI") {
            if !uri.trim().is_empty() {
                self.database.uri = Some(uri);
            }
        }
        if let Ok(domain) = std::env::var("MATRIX_BRIDGE_DINGTALK_DOMAIN") {
            if !domain.trim().is_empty() {
                self.bridge.domain = domain;
            }
        }
        if let Ok(homeserver_url) = std::env::var("MATRIX_BRIDGE_DINGTALK_HOMESERVER_URL") {
            if !homeserver_url.trim().is_empty() {
                self.bridge.homeserver_url = homeserver_url;
            }
        }
        if let Ok(bot_username) = std::env::var("MATRIX_BRIDGE_DINGTALK_BOT_USERNAME") {
            if !bot_username.trim().is_empty() {
                self.bridge.bot_username = bot_username;
            }
        }
        if let Ok(value) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_ENABLED")
            .or_else(|_| std::env::var("DINGTALK_STREAM_ENABLED"))
        {
            if let Some(enabled) = parse_bool_str(&value) {
                self.stream.enabled = enabled;
            }
        }
        if let Ok(client_id) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_CLIENT_ID")
            .or_else(|_| std::env::var("DINGTALK_CLIENT_ID"))
            .or_else(|_| std::env::var("DINGTALK_STREAM_CLIENT_ID"))
        {
            if !client_id.trim().is_empty() {
                self.stream.client_id = client_id;
            }
        }
        if let Ok(client_secret) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_CLIENT_SECRET")
            .or_else(|_| std::env::var("DINGTALK_CLIENT_SECRET"))
            .or_else(|_| std::env::var("DINGTALK_STREAM_CLIENT_SECRET"))
        {
            if !client_secret.trim().is_empty() {
                self.stream.client_secret = client_secret;
            }
        }
        if let Ok(openapi_host) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_OPENAPI_HOST")
            .or_else(|_| std::env::var("DINGTALK_STREAM_OPENAPI_HOST"))
        {
            if !openapi_host.trim().is_empty() {
                self.stream.openapi_host = openapi_host;
            }
        }
        if let Ok(value) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_KEEP_ALIVE_IDLE_SECS")
            .or_else(|_| std::env::var("DINGTALK_STREAM_KEEP_ALIVE_IDLE_SECS"))
        {
            if let Ok(parsed) = value.parse::<u64>() {
                self.stream.keep_alive_idle_secs = parsed.max(1);
            }
        }
        if let Ok(value) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_RECONNECT_INTERVAL_SECS")
            .or_else(|_| std::env::var("DINGTALK_STREAM_RECONNECT_INTERVAL_SECS"))
        {
            if let Ok(parsed) = value.parse::<u64>() {
                self.stream.reconnect_interval_secs = parsed.max(1);
            }
        }
        if let Ok(value) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_AUTO_RECONNECT")
            .or_else(|_| std::env::var("DINGTALK_STREAM_AUTO_RECONNECT"))
        {
            if let Some(enabled) = parse_bool_str(&value) {
                self.stream.auto_reconnect = enabled;
            }
        }
        if let Ok(local_ip) = std::env::var("MATRIX_BRIDGE_DINGTALK_STREAM_LOCAL_IP")
            .or_else(|_| std::env::var("DINGTALK_STREAM_LOCAL_IP"))
        {
            let trimmed = local_ip.trim();
            self.stream.local_ip = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
        }
        Ok(())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.bridge.domain.is_empty() {
            return Err(ConfigError::InvalidConfig(
                "bridge.domain is required".to_string(),
            ));
        }
        if self.bridge.homeserver_url.is_empty() {
            return Err(ConfigError::InvalidConfig(
                "bridge.homeserver_url is required".to_string(),
            ));
        }
        if self.database.connection_string().is_empty() {
            return Err(ConfigError::InvalidConfig(
                "database.url/database.uri/database.filename is required".to_string(),
            ));
        }
        if !self.bridge.username_template.contains("{{.}}")
            && !self.bridge.username_template.contains("{user_id}")
        {
            return Err(ConfigError::InvalidConfig(
                "bridge.username_template must contain '{{.}}' or '{user_id}' placeholder"
                    .to_string(),
            ));
        }
        if self.bridge.message_limit > 0 && self.bridge.message_cooldown == 0 {
            return Err(ConfigError::InvalidConfig(
                "bridge.message_cooldown must be > 0 when bridge.message_limit > 0".to_string(),
            ));
        }
        if self.stream.enabled {
            if self.stream.client_id.trim().is_empty() {
                return Err(ConfigError::InvalidConfig(
                    "stream.client_id is required when stream.enabled is true".to_string(),
                ));
            }
            if self.stream.client_secret.trim().is_empty() {
                return Err(ConfigError::InvalidConfig(
                    "stream.client_secret is required when stream.enabled is true".to_string(),
                ));
            }
            if self.stream.openapi_host.trim().is_empty() {
                return Err(ConfigError::InvalidConfig(
                    "stream.openapi_host cannot be empty when stream.enabled is true".to_string(),
                ));
            }
            if self.stream.keep_alive_idle_secs == 0 {
                return Err(ConfigError::InvalidConfig(
                    "stream.keep_alive_idle_secs must be > 0".to_string(),
                ));
            }
            if self.stream.reconnect_interval_secs == 0 {
                return Err(ConfigError::InvalidConfig(
                    "stream.reconnect_interval_secs must be > 0".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BridgeConfig {
    pub domain: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default)]
    pub homeserver_url: String,
    #[serde(default = "default_bot_username")]
    pub bot_username: String,
    #[serde(default = "default_bot_displayname")]
    pub bot_displayname: String,
    #[serde(default)]
    pub bot_avatar: String,
    #[serde(default = "default_matrix_username_template")]
    pub username_template: String,
    #[serde(default = "default_permissions")]
    pub permissions: HashMap<String, String>,
    #[serde(default = "default_matrix_displayname_template")]
    pub displayname_template: String,
    #[serde(default = "default_avatar_template")]
    pub avatar_template: String,
    #[serde(default = "default_true")]
    pub bridge_matrix_reply: bool,
    #[serde(default = "default_true")]
    pub bridge_matrix_edit: bool,
    #[serde(default)]
    pub bridge_matrix_reactions: bool,
    #[serde(default = "default_true")]
    pub bridge_matrix_redactions: bool,
    #[serde(default)]
    pub bridge_matrix_leave: bool,
    #[serde(default)]
    pub bridge_dingtalk_join: bool,
    #[serde(default)]
    pub bridge_dingtalk_leave: bool,
    #[serde(default = "default_true")]
    pub allow_plain_text: bool,
    #[serde(default = "default_true")]
    pub allow_markdown: bool,
    #[serde(default)]
    pub allow_html: bool,
    #[serde(default)]
    pub allow_images: bool,
    #[serde(default)]
    pub allow_videos: bool,
    #[serde(default)]
    pub allow_audio: bool,
    #[serde(default)]
    pub allow_files: bool,
    #[serde(default)]
    pub max_media_size: usize,
    #[serde(default = "default_message_limit")]
    pub message_limit: u32,
    #[serde(default = "default_message_cooldown")]
    pub message_cooldown: u64,
    #[serde(default)]
    pub blocked_matrix_msgtypes: Vec<String>,
    #[serde(default)]
    pub max_text_length: usize,
    #[serde(default = "default_true")]
    pub enable_failure_degrade: bool,
    #[serde(default = "default_failure_notice_template")]
    pub failure_notice_template: String,
    #[serde(default = "default_user_sync_interval_secs")]
    pub user_sync_interval_secs: u64,
    #[serde(default = "default_user_mapping_stale_ttl_hours")]
    pub user_mapping_stale_ttl_hours: u64,
    #[serde(default = "default_webhook_timeout")]
    pub webhook_timeout: u64,
    #[serde(default = "default_api_timeout")]
    pub api_timeout: u64,
    #[serde(default = "default_true")]
    pub enable_rich_text: bool,
    #[serde(default)]
    pub convert_cards: bool,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            domain: String::new(),
            port: default_port(),
            bind_address: default_bind_address(),
            homeserver_url: String::new(),
            bot_username: default_bot_username(),
            bot_displayname: default_bot_displayname(),
            bot_avatar: String::new(),
            username_template: default_matrix_username_template(),
            permissions: default_permissions(),
            displayname_template: default_matrix_displayname_template(),
            avatar_template: default_avatar_template(),
            bridge_matrix_reply: true,
            bridge_matrix_edit: true,
            bridge_matrix_reactions: false,
            bridge_matrix_redactions: true,
            bridge_matrix_leave: false,
            bridge_dingtalk_join: false,
            bridge_dingtalk_leave: false,
            allow_plain_text: true,
            allow_markdown: true,
            allow_html: false,
            allow_images: false,
            allow_videos: false,
            allow_audio: false,
            allow_files: false,
            max_media_size: 0,
            message_limit: default_message_limit(),
            message_cooldown: default_message_cooldown(),
            blocked_matrix_msgtypes: Vec::new(),
            max_text_length: 0,
            enable_failure_degrade: true,
            failure_notice_template: default_failure_notice_template(),
            user_sync_interval_secs: default_user_sync_interval_secs(),
            user_mapping_stale_ttl_hours: default_user_mapping_stale_ttl_hours(),
            webhook_timeout: default_webhook_timeout(),
            api_timeout: default_api_timeout(),
            enable_rich_text: true,
            convert_cards: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegistrationConfig {
    #[serde(alias = "id")]
    pub bridge_id: String,
    #[serde(default, alias = "as_token")]
    pub appservice_token: String,
    #[serde(default, alias = "hs_token")]
    pub homeserver_token: String,
    #[serde(default = "default_sender_localpart")]
    pub sender_localpart: String,
    #[serde(default)]
    pub namespaces: RegistrationNamespaces,
    #[serde(default)]
    pub rate_limited: bool,
    #[serde(
        default = "default_registration_protocols",
        alias = "protocol",
        deserialize_with = "deserialize_registration_protocols"
    )]
    pub protocols: Vec<String>,
}

impl Default for RegistrationConfig {
    fn default() -> Self {
        Self {
            bridge_id: String::new(),
            appservice_token: String::new(),
            homeserver_token: String::new(),
            sender_localpart: default_sender_localpart(),
            namespaces: RegistrationNamespaces::default(),
            rate_limited: false,
            protocols: default_registration_protocols(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct RegistrationNamespaces {
    #[serde(default)]
    pub users: Vec<RegistrationNamespaceEntry>,
    #[serde(default)]
    pub aliases: Vec<RegistrationNamespaceEntry>,
    #[serde(default)]
    pub rooms: Vec<RegistrationNamespaceEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct RegistrationNamespaceEntry {
    #[serde(default)]
    pub exclusive: bool,
    #[serde(default)]
    pub regex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub webhooks: HashMap<String, String>,
    #[serde(default)]
    pub security: SecurityConfig,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            webhooks: HashMap::new(),
            security: SecurityConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    #[serde(default = "default_security_type", alias = "type")]
    pub security_type: String,
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub secret: Option<String>,
    #[serde(default)]
    pub ip_whitelist: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            security_type: default_security_type(),
            keyword: None,
            secret: None,
            ip_whitelist: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(alias = "console", alias = "min_level", default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_line_date_format")]
    pub line_date_format: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub files: Vec<LoggingFileConfig>,
    #[serde(default)]
    pub writers: Vec<LoggingWriterConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingFileConfig {
    pub file: String,
    #[serde(default = "default_log_file_level")]
    pub level: String,
    #[serde(default = "default_log_max_files")]
    pub max_files: String,
    #[serde(default = "default_log_max_size")]
    pub max_size: String,
    #[serde(default = "default_log_date_pattern")]
    pub date_pattern: String,
    #[serde(default)]
    pub enabled: Vec<String>,
    #[serde(default)]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingWriterConfig {
    #[serde(default = "default_log_writer_type")]
    pub r#type: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub max_size: Option<u64>,
    #[serde(default)]
    pub max_backups: Option<u64>,
    #[serde(default)]
    pub compress: Option<bool>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            line_date_format: default_line_date_format(),
            format: default_log_format(),
            file: None,
            files: Vec::new(),
            writers: vec![LoggingWriterConfig {
                r#type: default_log_writer_type(),
                format: default_log_format(),
                filename: None,
                max_size: None,
                max_backups: None,
                compress: None,
            }],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub conn_string: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub max_open_conns: Option<u32>,
    #[serde(default)]
    pub max_idle_conns: Option<u32>,
    #[serde(default)]
    pub max_connections: Option<u32>,
    #[serde(default)]
    pub min_connections: Option<u32>,
}

impl DatabaseConfig {
    pub fn db_type(&self) -> DbType {
        if let Some(db_type) = self
            .r#type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return match db_type.to_ascii_lowercase().as_str() {
                "sqlite" => DbType::Sqlite,
                "mysql" | "mariadb" => DbType::Mysql,
                _ => DbType::Postgres,
            };
        }

        let url = self.connection_string();
        if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
            DbType::Sqlite
        } else if url.starts_with("mysql://") || url.starts_with("mariadb://") {
            DbType::Mysql
        } else {
            DbType::Postgres
        }
    }

    pub fn connection_string(&self) -> String {
        if let Some(ref url) = self.url {
            url.clone()
        } else if let Some(ref uri) = self.uri {
            uri.clone()
        } else if let Some(ref conn) = self.conn_string {
            conn.clone()
        } else if let Some(ref file) = self.filename {
            format!("sqlite://{}", file)
        } else {
            String::new()
        }
    }

    pub fn sqlite_path(&self) -> Option<String> {
        if let DbType::Sqlite = self.db_type() {
            let url = self.connection_string();
            Some(url.strip_prefix("sqlite://").unwrap_or(&url).to_string())
        } else {
            None
        }
    }

    pub fn max_connections(&self) -> Option<u32> {
        match self.db_type() {
            DbType::Postgres | DbType::Mysql => self.max_connections.or(self.max_open_conns),
            DbType::Sqlite => Some(
                self.max_connections
                    .or(self.max_open_conns)
                    .unwrap_or(1)
                    .max(1),
            ),
        }
    }

    pub fn min_connections(&self) -> Option<u32> {
        match self.db_type() {
            DbType::Postgres | DbType::Mysql => self.min_connections.or(self.max_idle_conns),
            DbType::Sqlite => Some(
                self.min_connections
                    .or(self.max_idle_conns)
                    .unwrap_or(1)
                    .max(1),
            ),
        }
    }

    pub fn db_type_name(&self) -> &'static str {
        match self.db_type() {
            DbType::Postgres => "postgres",
            DbType::Sqlite => "sqlite",
            DbType::Mysql => "mysql",
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            r#type: Some("sqlite".to_string()),
            uri: Some("sqlite://./dingtalk.db".to_string()),
            url: None,
            conn_string: None,
            filename: None,
            max_open_conns: Some(10),
            max_idle_conns: Some(1),
            max_connections: Some(10),
            min_connections: Some(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbType {
    Postgres,
    Sqlite,
    Mysql,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoomConfig {
    #[serde(default)]
    pub default_visibility: String,
    #[serde(default)]
    pub room_alias_prefix: String,
    #[serde(default)]
    pub enable_room_creation: bool,
    #[serde(default = "default_kick_for")]
    pub kick_for: u64,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            default_visibility: "private".to_string(),
            room_alias_prefix: "_dingtalk_".to_string(),
            enable_room_creation: true,
            kick_for: default_kick_for(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelConfig {
    #[serde(default = "default_channel_name_pattern")]
    pub name_pattern: String,
    #[serde(default)]
    pub enable_channel_creation: bool,
    #[serde(default)]
    pub topic_format: String,
    #[serde(default)]
    pub delete_options: ChannelDeleteOptionsConfig,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            name_pattern: default_channel_name_pattern(),
            enable_channel_creation: true,
            topic_format: "Bridged from Matrix room {room_id}".to_string(),
            delete_options: ChannelDeleteOptionsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChannelDeleteOptionsConfig {
    #[serde(default)]
    pub name_prefix: Option<String>,
    #[serde(default)]
    pub topic_prefix: Option<String>,
    #[serde(default)]
    pub disable_messaging: bool,
    #[serde(default = "default_unset_room_alias")]
    pub unset_room_alias: bool,
    #[serde(default = "default_unlist_from_directory")]
    pub unlist_from_directory: bool,
    #[serde(default = "default_set_invite_only")]
    pub set_invite_only: bool,
    #[serde(default = "default_ghosts_leave")]
    pub ghosts_leave: bool,
}

impl Default for ChannelDeleteOptionsConfig {
    fn default() -> Self {
        Self {
            name_prefix: None,
            topic_prefix: None,
            disable_messaging: false,
            unset_room_alias: default_unset_room_alias(),
            unlist_from_directory: default_unlist_from_directory(),
            set_invite_only: default_set_invite_only(),
            ghosts_leave: default_ghosts_leave(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitsConfig {
    #[serde(default = "default_room_ghost_join_delay")]
    pub room_ghost_join_delay: u64,
    #[serde(default = "default_dingtalk_send_delay")]
    pub dingtalk_send_delay: u64,
    #[serde(default = "default_room_count")]
    pub room_count: i32,
    #[serde(default = "default_matrix_event_age_limit_ms")]
    pub matrix_event_age_limit_ms: u64,
    #[serde(default = "default_dingtalk_rate_limit_per_minute")]
    pub dingtalk_rate_limit_per_minute: u32,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            room_ghost_join_delay: default_room_ghost_join_delay(),
            dingtalk_send_delay: default_dingtalk_send_delay(),
            room_count: default_room_count(),
            matrix_event_age_limit_ms: default_matrix_event_age_limit_ms(),
            dingtalk_rate_limit_per_minute: default_dingtalk_rate_limit_per_minute(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GhostsConfig {
    #[serde(default = "default_nick_pattern")]
    pub nick_pattern: String,
    #[serde(default = "default_username_pattern")]
    pub username_pattern: String,
    #[serde(default = "default_username_template")]
    pub username_template: String,
    #[serde(default = "default_displayname_template")]
    pub displayname_template: String,
    #[serde(default)]
    pub avatar_url_template: Option<String>,
}

impl Default for GhostsConfig {
    fn default() -> Self {
        Self {
            nick_pattern: default_nick_pattern(),
            username_pattern: default_username_pattern(),
            username_template: default_username_template(),
            displayname_template: default_displayname_template(),
            avatar_url_template: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StreamConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default = "default_stream_openapi_host")]
    pub openapi_host: String,
    #[serde(default = "default_stream_keep_alive_idle_secs")]
    pub keep_alive_idle_secs: u64,
    #[serde(default = "default_stream_reconnect_interval_secs")]
    pub reconnect_interval_secs: u64,
    #[serde(default = "default_true")]
    pub auto_reconnect: bool,
    #[serde(default)]
    pub local_ip: Option<String>,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            client_id: String::new(),
            client_secret: String::new(),
            openapi_host: default_stream_openapi_host(),
            keep_alive_idle_secs: default_stream_keep_alive_idle_secs(),
            reconnect_interval_secs: default_stream_reconnect_interval_secs(),
            auto_reconnect: true,
            local_ip: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallbackConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_callback_port")]
    pub port: u16,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub aes_key: Option<String>,
}

impl Default for CallbackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_callback_port(),
            bind_address: default_bind_address(),
            token: String::new(),
            aes_key: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: default_metrics_port(),
            bind_address: default_bind_address(),
        }
    }
}

fn default_port() -> u16 {
    9006
}

fn default_bind_address() -> String {
    "0.0.0.0".to_string()
}

fn default_bot_username() -> String {
    "_dingtalk_bot".to_string()
}

fn default_bot_displayname() -> String {
    "DingTalk Bridge".to_string()
}

fn default_matrix_username_template() -> String {
    "dingtalk_{{.}}".to_string()
}

fn default_matrix_displayname_template() -> String {
    "{{.}} (DingTalk)".to_string()
}

fn default_avatar_template() -> String {
    String::new()
}

fn default_permissions() -> HashMap<String, String> {
    let mut permissions = HashMap::new();
    permissions.insert("*".to_string(), "relay".to_string());
    permissions
}

fn default_true() -> bool {
    true
}

fn default_message_limit() -> u32 {
    60
}

fn default_message_cooldown() -> u64 {
    1000
}

fn default_failure_notice_template() -> String {
    "[bridge degraded] failed to deliver message from Matrix event {matrix_event_id}: {error}"
        .to_string()
}

fn default_user_sync_interval_secs() -> u64 {
    300
}

fn default_user_mapping_stale_ttl_hours() -> u64 {
    24 * 30
}

fn default_webhook_timeout() -> u64 {
    30
}

fn default_api_timeout() -> u64 {
    60
}

fn default_sender_localpart() -> String {
    "_dingtalk_bot".to_string()
}

fn default_registration_protocols() -> Vec<String> {
    vec!["dingtalk".to_string()]
}

fn default_security_type() -> String {
    "sign".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_writer_type() -> String {
    "stdout".to_string()
}

fn default_line_date_format() -> String {
    "MMM-D HH:mm:ss.SSS".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_log_file_level() -> String {
    "info".to_string()
}

fn default_log_max_files() -> String {
    "14".to_string()
}

fn default_log_max_size() -> String {
    "50".to_string()
}

fn default_log_date_pattern() -> String {
    "daily".to_string()
}

fn default_kick_for() -> u64 {
    30000
}

fn default_channel_name_pattern() -> String {
    "[DingTalk] :name".to_string()
}

fn default_unset_room_alias() -> bool {
    true
}

fn default_unlist_from_directory() -> bool {
    true
}

fn default_set_invite_only() -> bool {
    true
}

fn default_ghosts_leave() -> bool {
    true
}

fn default_room_ghost_join_delay() -> u64 {
    6000
}

fn default_dingtalk_send_delay() -> u64 {
    1500
}

fn default_room_count() -> i32 {
    -1
}

fn default_matrix_event_age_limit_ms() -> u64 {
    900000
}

fn default_dingtalk_rate_limit_per_minute() -> u32 {
    20
}

fn default_nick_pattern() -> String {
    ":nick".to_string()
}

fn default_username_pattern() -> String {
    ":username".to_string()
}

fn default_username_template() -> String {
    "_dingtalk_{user_id}".to_string()
}

fn default_displayname_template() -> String {
    "{username}".to_string()
}

fn default_callback_port() -> u16 {
    9007
}

fn default_metrics_port() -> u16 {
    9008
}

fn default_stream_openapi_host() -> String {
    "https://api.dingtalk.com".to_string()
}

fn default_stream_keep_alive_idle_secs() -> u64 {
    120
}

fn default_stream_reconnect_interval_secs() -> u64 {
    3
}

fn parse_bool_str(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn deserialize_registration_protocols<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(vec![s])
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn parse_new_example_config() {
        let config = Config::load_from_bytes(include_bytes!("../../config/config.example.yaml"))
            .expect("new example config should parse");

        assert_eq!(config.bridge.domain, "127.0.0.1:8008");
        assert_eq!(config.bridge.bot_username, "_dingtalk_bot");
        assert_eq!(config.database.db_type_name(), "sqlite");
        assert_eq!(
            config.database.connection_string(),
            "sqlite://./dingtalk.db"
        );
        assert_eq!(config.bridge.message_limit, 60);
        assert_eq!(config.bridge.message_cooldown, 1000);
        assert!(config.bridge.permissions.contains_key("*"));
        assert!(config.stream.enabled);
        assert_eq!(
            config.stream.client_id,
            "CHANGE_ME_DINGTALK_CLIENT_ID".to_string()
        );
    }
}
