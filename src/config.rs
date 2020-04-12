use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};

#[derive(Serialize, Deserialize, Debug)]
pub struct App {
    #[serde(default)]
    listener: Listener,
    sensors: HashMap<String, Sensor>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct Listener {
    #[serde(default = "Listener::default_ip")]
    ip: IpAddr,

    #[serde(default = "Listener::default_port")]
    port: u16,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
enum Sensor {
    Dht11 { pin: u64 },
    Dht22 { pin: u64 },
}

impl Listener {
    pub fn default_ip() -> IpAddr {
        IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))
    }

    pub fn default_port() -> u16 {
        9742
    }

    pub fn socket_addr(self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port)
    }
}

impl Into<SocketAddr> for Listener {
    fn into(self) -> SocketAddr {
        self.socket_addr()
    }
}

impl Default for Listener {
    fn default() -> Self {
        Self {
            ip: Self::default_ip(),
            port: Self::default_port(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    #[test]
    fn basic_toml_with_listener() {
        let expected_ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let expected_port = 42069;
        let foo_pin = 2;
        let bar_pin = 5;
        let toml = format!(
            "
            [listener]
            ip = '{ip}'\n\
            port = {port}\n\
            \n\
            [sensors]\n\
            foo = {{ type = \"DHT11\", pin = {foo_pin} }}\n\
            bar = {{ type = \"DHT22\", pin = {bar_pin} }}\n\
            ",
            ip = expected_ip,
            port = expected_port,
            foo_pin = foo_pin,
            bar_pin = bar_pin,
        );
        let config: App = toml::from_str(toml.as_str()).unwrap();
        let sock = config.listener.socket_addr();
        assert_eq!(sock.ip(), expected_ip);
        assert_eq!(sock.port(), expected_port);
        let expected_sensors: HashMap<_, _> = vec![
            (String::from("foo"), Sensor::Dht11 { pin: foo_pin }),
            (String::from("bar"), Sensor::Dht22 { pin: bar_pin }),
        ]
        .into_iter()
        .collect();

        assert_eq!(config.sensors, expected_sensors)
    }

    #[test]
    fn basic_toml_no_listener() {
        let foo_pin = 2;
        let bar_pin = 5;
        let toml = format!(
            "
            [sensors]\n\
            foo = {{ type = \"DHT11\", pin = {foo_pin} }}\n\
            bar = {{ type = \"DHT22\", pin = {bar_pin} }}\n\
            ",
            foo_pin = foo_pin,
            bar_pin = bar_pin,
        );
        let config: App = toml::from_str(toml.as_str()).unwrap();
        assert_eq!(config.listener, Listener::default());
        let expected_sensors: HashMap<_, _> = vec![
            (String::from("foo"), Sensor::Dht11 { pin: foo_pin }),
            (String::from("bar"), Sensor::Dht22 { pin: bar_pin }),
        ]
        .into_iter()
        .collect();

        assert_eq!(config.sensors, expected_sensors)
    }
}
