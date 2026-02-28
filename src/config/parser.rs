use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize};

use super::ConfigError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub bridge: BridgeConfig,
    #[serde(default)]
    pub registration: RegistrationConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
    pub database: DatabaseConfig,
    pub room: RoomConfig,
    pub channel: ChannelConfig,
    #[serde(default)]
    pub limits: LimitsConfig,
    pub ghosts: GhostsConfig,
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
                "database.url or database.filename is required".to_string(),
            ));
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
    #[serde(default = "default_presence_interval")]
    pub presence_interval: u64,
    #[serde(default)]
    pub disable_presence: bool,
    #[serde(default)]
    pub disable_typing_notifications: bool,
    #[serde(default)]
    pub disable_deletion_forwarding: bool,
    #[serde(default)]
    pub enable_self_service_bridging: bool,
    #[serde(default)]
    pub disable_portal_bridging: bool,
    #[serde(default)]
    pub disable_read_receipts: bool,
    #[serde(default)]
    pub disable_join_leave_notifications: bool,
    #[serde(default)]
    pub disable_invite_notifications: bool,
    #[serde(default)]
    pub disable_room_topic_notifications: bool,
    #[serde(default)]
    pub user_limit: Option<u32>,
    #[serde(default)]
    pub admin_mxid: Option<String>,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    #[serde(default = "default_security_type")]
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
    #[serde(alias = "console", default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_line_date_format")]
    pub line_date_format: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub files: Vec<LoggingFileConfig>,
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
pub struct DatabaseConfig {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub conn_string: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub max_connections: Option<u32>,
    #[serde(default)]
    pub min_connections: Option<u32>,
}

impl DatabaseConfig {
    pub fn db_type(&self) -> DbType {
        let url = self.connection_string();
        if url.starts_with("sqlite://") {
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
            DbType::Postgres | DbType::Mysql => self.max_connections,
            DbType::Sqlite => Some(1),
        }
    }

    pub fn min_connections(&self) -> Option<u32> {
        match self.db_type() {
            DbType::Postgres | DbType::Mysql => self.min_connections,
            DbType::Sqlite => Some(1),
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

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

fn default_presence_interval() -> u64 {
    500
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

fn deserialize_registration_protocols<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(vec![s])
}
