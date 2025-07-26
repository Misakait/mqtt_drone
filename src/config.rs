use std::env;

/// 应用程序配置结构
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mongodb_uri: String,
    pub mqtt_host: String,
    pub mqtt_port: u16,
    pub mqtt_username: String,
    pub mqtt_password: String,
    pub ca_cert_path: String,
}

impl AppConfig {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let mongodb_uri = env::var("MONGODB_URI")?;
        let mqtt_host = env::var("MQTT_HOST")?;
        let mqtt_port_str = env::var("MQTT_PORT")?;
        let mqtt_port: u16 = mqtt_port_str.parse()?;
        let mqtt_username = env::var("MQTT_USERNAME")?;
        let mqtt_password = env::var("MQTT_PASSWORD")?;
        let ca_cert_path = env::var("CA_CERT_PATH")?;

        Ok(Self {
            mongodb_uri,
            mqtt_host,
            mqtt_port,
            mqtt_username,
            mqtt_password,
            ca_cert_path,
        })
    }
}
