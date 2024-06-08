use core::{
    net::{IpAddr, Ipv4Addr},
    str::FromStr,
};

use base64::{engine::general_purpose, Engine};
use derive_builder::Builder;
use rand::prelude::*;
use semver::Version;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The handshake message structure used by an Agent to communicate with the Server.
#[derive(Builder, Default, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[builder(default, try_setter, setter(into))]
pub struct AgentHandshake {
    /// Unique identifier for the Agent.
    pub agent_id: AgentId,

    /// Name of the Agent.
    #[builder(setter(strip_option))]
    pub agent_name: Option<String>,

    /// Authentication information for the Agent, can be a key or anonymous.
    pub auth: Auth,

    /// The version of the Agent software.
    #[builder(setter(strip_option, custom))]
    pub version: Option<Version>,

    /// Local network information of the Agent.
    #[builder(setter(strip_option))]
    pub local_info: Option<LocalInfo>,

    /// Information about the service the Agent is providing or connecting to.
    #[builder(setter(strip_option))]
    pub service_info: Option<ServiceInfo>,

    /// Encryption information for secure communication.
    #[builder(setter(strip_option))]
    pub encryption: Option<Encryption>,

    /// Interval in milliseconds for the Agent to send a heartbeat message.
    pub heartbeat_interval: u32,

    /// Timestamp of the handshake message.
    pub timestamp: u64,
}

impl AgentHandshake {
    pub fn builder() -> AgentHandshakeBuilder {
        AgentHandshakeBuilder::default()
    }
}

impl AgentHandshakeBuilder {
    pub fn generate_agent_id(&mut self) -> &mut Self {
        self.agent_id = Some(AgentId::default());
        self
    }

    pub fn version(&mut self, version: &str) -> &mut Self {
        self.version = Some(Some(version.parse().unwrap()));
        self
    }
}

/// Unique identifier for an Agent.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct AgentId(Uuid);

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        Uuid::from_str(s).unwrap().into()
    }
}

impl From<Uuid> for AgentId {
    fn from(u: Uuid) -> Self {
        AgentId(u)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Uuid::new_v4().into()
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Authentication information for the Agent.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub enum Auth {
    /// Authentication using a secret key.
    Key(String),
    /// Anonymous authentication.
    #[default]
    Anonymous,
}

impl From<&str> for Auth {
    fn from(s: &str) -> Self {
        Auth::Key(s.into())
    }
}

/// Local network information of the Agent.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LocalInfo {
    /// Local IP address of the Agent.
    ip: IpAddr,
    /// Local port the Agent is listening on.
    port: u16,
}

impl Default for LocalInfo {
    fn default() -> Self {
        LocalInfo {
            ip: Ipv4Addr::new(127, 0, 0, 1).into(),
            port: 8080,
        }
    }
}

/// Information about the service the Agent is providing or connecting to.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ServiceInfo {
    /// IP address of the target service.
    target_ip: IpAddr,
    /// Port of the target service.
    target_port: u16,
}

impl Default for ServiceInfo {
    fn default() -> Self {
        ServiceInfo {
            target_ip: Ipv4Addr::new(127, 0, 0, 1).into(),
            target_port: 8080,
        }
    }
}

/// Encryption information for secure communication.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Encryption {
    /// Encryption method used.
    method: String,
    /// Public key used for encryption.
    key: String,
}

#[cfg(test)]
mod tests {
    use core::net::Ipv4Addr;

    use super::*;

    #[test]
    fn test_agent_handshake_serialization() {
        let agent_id = "agent-123".into();
        let agent_name = None;
        let auth = Auth::Key("secret-key".into());
        let version = Some("1.0.0".parse::<Version>().unwrap());
        let local_info = Some(LocalInfo {
            ip: Ipv4Addr::new(127, 0, 0, 1).into(),
            port: 8080,
        });
        let service_info = Some(ServiceInfo {
            target_ip: Ipv4Addr::new(192, 168, 0, 1).into(),
            target_port: 9000,
        });
        let encryption = Some(Encryption {
            method: "AES-256".into(),
            key: "encryption-key".into(),
        });
        let heartbeat_interval = 5000;
        let timestamp = 1631234567890;

        let handshake = AgentHandshake {
            agent_id,
            agent_name,
            auth,
            version,
            local_info,
            service_info,
            encryption,
            heartbeat_interval,
            timestamp,
        };

        let serialized = serde_json::to_string(&handshake).unwrap();
        let deserialized: AgentHandshake = serde_json::from_str(&serialized).unwrap();

        println!("{} - {}", serialized.len(), serialized);
        println!("{:?}", deserialized);

        assert_eq!(handshake, deserialized);
    }

    #[test]
    fn test_agent_handshake_builder() {
        let handshake = AgentHandshake::builder()
            .agent_id("agent-123")
            .auth("secret-key")
            .version("1.0.0")
            .local_info(LocalInfo {
                ip: Ipv4Addr::new(127, 0, 0, 1).into(),
                port: 8080,
            })
            .service_info(ServiceInfo {
                target_ip: Ipv4Addr::new(192, 168, 0, 1).into(),
                target_port: 9000,
            })
            .encryption(Encryption {
                method: "AES-256".into(),
                key: "encryption-key".into(),
            })
            .heartbeat_interval(5000u32)
            .timestamp(1631234567890u64)
            .build()
            .unwrap();

        assert_eq!(handshake.agent_id.0, "agent-123");
    }

    #[test]
    fn test_builder_with_default() {
        let handshake = AgentHandshake::builder().build().unwrap();
        println!("{:?}", handshake);
    }
}