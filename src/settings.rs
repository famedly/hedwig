use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Settings {
    pub fcm_admin_key: String,
    pub server_port: u16,
    pub app_id: String,
    pub fcm_collapse_key: String,
    pub fcm_notification_title: String,
    pub fcm_notification_body: String,
    pub fcm_notification_sound: String,
    pub fcm_notification_icon: String,
    pub fcm_notification_tag: String,
    pub fcm_notification_android_channel_id: String,
    pub fcm_notification_click_action: String,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut conf = Config::new();

        conf.merge(File::with_name("config.toml"))?;
        conf.merge(Environment::with_prefix("push_gw").separator("_"))?;
        conf.try_into()
    }
}
