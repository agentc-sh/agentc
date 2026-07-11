// SPDX-FileCopyrightText: 2026 agentc Authors
//
// SPDX-License-Identifier: MIT

use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::Error as DeError,
    ser::{SerializeMap, SerializeStruct},
};
use serde_json::{Value, from_value};
use std::collections::HashMap;
use utoipa::ToSchema;

pub type ProtocolVersion = String;
pub type TransportProtocol = String;
pub type SecurityRequirement = HashMap<String, Vec<String>>;

pub const TRANSPORT_PROTOCOL_GRPC: &str = "GRPC";

fn normalize_agent_interface_url(url: String, protocol_binding: &str) -> String {
    if protocol_binding.eq_ignore_ascii_case(TRANSPORT_PROTOCOL_GRPC) {
        if let Some(stripped) = url.strip_prefix("http://") {
            return stripped.to_string();
        }
    }

    url
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub version: String,
    pub supported_interfaces: Vec<AgentInterface>,
    pub capabilities: AgentCapabilities,
    pub default_input_modes: Vec<String>,
    pub default_output_modes: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_vec_null_as_default")]
    pub skills: Vec<AgentSkill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<AgentProvider>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<HashMap<String, SecurityScheme>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_security_requirements"
    )]
    pub security_requirements: Option<Vec<SecurityRequirement>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Vec<AgentCardSignature>>,
}

fn deserialize_vec_null_as_default<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_optional_security_requirements<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<SecurityRequirement>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<Vec<Value>>::deserialize(deserializer)?;

    raw.map(|items| {
        items
            .into_iter()
            .map(parse_security_requirement_value)
            .collect()
    })
    .transpose()
}

fn parse_security_requirement_value<E>(value: Value) -> Result<SecurityRequirement, E>
where
    E: DeError,
{
    if let Ok(requirement) = from_value::<SecurityRequirement>(value.clone()) {
        return Ok(requirement);
    }

    let Value::Object(mut object) = value else {
        return Err(E::custom("security requirement must be an object"));
    };

    if let Some(schemes) = object.remove("schemes") {
        return parse_security_requirement_map::<E>(schemes);
    }

    Err(E::custom("invalid security requirement shape"))
}

fn parse_security_requirement_map<E>(value: Value) -> Result<SecurityRequirement, E>
where
    E: DeError,
{
    let Value::Object(object) = value else {
        return Err(E::custom("security requirement schemes must be an object"));
    };

    let mut requirement = HashMap::new();

    for (scheme, scopes_value) in object {
        let scopes = match scopes_value {
            Value::Array(_) => from_value::<Vec<String>>(scopes_value).map_err(|error| {
                E::custom(format!("invalid security scopes for {scheme}: {error}"))
            })?,
            Value::Object(mut wrapped) => {
                let Some(list) = wrapped.remove("list") else {
                    return Err(E::custom(format!("invalid wrapped security scopes for {scheme}")));
                };

                from_value::<Vec<String>>(list).map_err(|error| {
                    E::custom(format!("invalid wrapped security scopes for {scheme}: {error}"))
                })?
            }
            _ => {
                return Err(E::custom(format!("security scopes for {scheme} must be a list")));
            }
        };

        requirement.insert(scheme, scopes);
    }

    Ok(requirement)
}

impl<'de> Deserialize<'de> for AgentCard {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct AgentCardSerde {
            name: String,
            description: String,
            version: String,
            supported_interfaces: Vec<AgentInterface>,
            capabilities: AgentCapabilities,
            default_input_modes: Vec<String>,
            default_output_modes: Vec<String>,
            #[serde(default, deserialize_with = "deserialize_vec_null_as_default")]
            skills: Vec<AgentSkill>,
            #[serde(default)]
            provider: Option<AgentProvider>,
            #[serde(default)]
            documentation_url: Option<String>,
            #[serde(default)]
            icon_url: Option<String>,
            #[serde(default)]
            security_schemes: Option<HashMap<String, SecurityScheme>>,
            #[serde(
                default,
                deserialize_with = "deserialize_optional_security_requirements"
            )]
            security_requirements: Option<Vec<SecurityRequirement>>,
            #[serde(default)]
            signatures: Option<Vec<AgentCardSignature>>,
        }

        let card = AgentCardSerde::deserialize(deserializer)?;

        Ok(Self {
            name: card.name,
            description: card.description,
            version: card.version,
            supported_interfaces: card.supported_interfaces,
            capabilities: card.capabilities,
            default_input_modes: card.default_input_modes,
            default_output_modes: card.default_output_modes,
            skills: card.skills,
            provider: card.provider,
            documentation_url: card.documentation_url,
            icon_url: card.icon_url,
            security_schemes: card.security_schemes,
            security_requirements: card.security_requirements,
            signatures: card.signatures,
        })
    }
}

#[derive(Debug, Clone, PartialEq, ToSchema)]
pub struct AgentInterface {
    pub url: String,
    #[serde(rename = "protocolBinding")]
    pub protocol_binding: TransportProtocol,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: ProtocolVersion,
    #[serde(default)]
    pub tenant: Option<String>,
}

impl AgentInterface {
    pub fn new(url: impl Into<String>, protocol_binding: impl Into<String>) -> Self {
        let protocol_binding = protocol_binding.into();

        Self {
            url: normalize_agent_interface_url(url.into(), &protocol_binding),
            protocol_binding,
            protocol_version: "1.0".to_string(),
            tenant: None,
        }
    }

    pub fn wire_url(&self) -> String {
        normalize_agent_interface_url(self.url.clone(), &self.protocol_binding)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentInterfaceSerde {
    url: String,
    protocol_binding: TransportProtocol,
    protocol_version: ProtocolVersion,
    #[serde(default)]
    tenant: Option<String>,
}

impl Serialize for AgentInterface {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer
            .serialize_struct("AgentInterface", if self.tenant.is_some() { 4 } else { 3 })?;
        state.serialize_field("url", &self.wire_url())?;
        state.serialize_field("protocolBinding", &self.protocol_binding)?;
        state.serialize_field("protocolVersion", &self.protocol_version)?;

        if let Some(tenant) = &self.tenant {
            state.serialize_field("tenant", tenant)?;
        }

        state.end()
    }
}

impl<'de> Deserialize<'de> for AgentInterface {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = AgentInterfaceSerde::deserialize(deserializer)?;

        Ok(Self {
            url: raw.url,
            protocol_binding: raw.protocol_binding,
            protocol_version: raw.protocol_version,
            tenant: raw.tenant,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<AgentExtension>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_agent_card: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentExtension {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    pub organization: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_modes: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_modes: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_security_requirements"
    )]
    pub security_requirements: Option<Vec<SecurityRequirement>>,
}

#[derive(Debug, Clone, PartialEq, ToSchema)]
pub enum SecurityScheme {
    ApiKey(ApiKeySecurityScheme),
    HttpAuth(HttpAuthSecurityScheme),
    OAuth2(OAuth2SecurityScheme),
    OpenIdConnect(OpenIdConnectSecurityScheme),
    MutualTls(MutualTlsSecurityScheme),
}

impl Serialize for SecurityScheme {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Self::ApiKey(value) => map.serialize_entry("apiKeySecurityScheme", value)?,
            Self::HttpAuth(value) => map.serialize_entry("httpAuthSecurityScheme", value)?,
            Self::OAuth2(value) => map.serialize_entry("oauth2SecurityScheme", value)?,
            Self::OpenIdConnect(value) => {
                map.serialize_entry("openIdConnectSecurityScheme", value)?
            }
            Self::MutualTls(value) => map.serialize_entry("mtlsSecurityScheme", value)?,
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for SecurityScheme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = HashMap::<String, Value>::deserialize(deserializer)?;

        if let Some(value) = raw.get("apiKeySecurityScheme") {
            return Ok(Self::ApiKey(from_value(value.clone()).map_err(DeError::custom)?));
        }

        if let Some(value) = raw.get("httpAuthSecurityScheme") {
            return Ok(Self::HttpAuth(from_value(value.clone()).map_err(DeError::custom)?));
        }

        if let Some(value) = raw.get("oauth2SecurityScheme") {
            return Ok(Self::OAuth2(from_value(value.clone()).map_err(DeError::custom)?));
        }

        if let Some(value) = raw.get("openIdConnectSecurityScheme") {
            return Ok(Self::OpenIdConnect(from_value(value.clone()).map_err(DeError::custom)?));
        }

        if let Some(value) = raw.get("mtlsSecurityScheme") {
            return Ok(Self::MutualTls(from_value(value.clone()).map_err(DeError::custom)?));
        }

        Err(DeError::custom("unknown security scheme variant"))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurityScheme {
    pub location: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpAuthSecurityScheme {
    pub scheme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2SecurityScheme {
    pub flows: OAuthFlows,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth2_metadata_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OpenIdConnectSecurityScheme {
    pub open_id_connect_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MutualTlsSecurityScheme {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, ToSchema)]
pub enum OAuthFlows {
    AuthorizationCode(AuthorizationCodeOAuthFlow),
    ClientCredentials(ClientCredentialsOAuthFlow),
    DeviceCode(DeviceCodeOAuthFlow),
    Implicit(ImplicitOAuthFlow),
    Password(PasswordOAuthFlow),
}

impl Serialize for OAuthFlows {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Self::AuthorizationCode(value) => map.serialize_entry("authorizationCode", value)?,
            Self::ClientCredentials(value) => map.serialize_entry("clientCredentials", value)?,
            Self::DeviceCode(value) => map.serialize_entry("deviceCode", value)?,
            Self::Implicit(value) => map.serialize_entry("implicit", value)?,
            Self::Password(value) => map.serialize_entry("password", value)?,
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for OAuthFlows {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = HashMap::<String, Value>::deserialize(deserializer)?;

        if let Some(value) = raw.get("authorizationCode") {
            return Ok(Self::AuthorizationCode(
                from_value(value.clone()).map_err(DeError::custom)?,
            ));
        }

        if let Some(value) = raw.get("clientCredentials") {
            return Ok(Self::ClientCredentials(
                from_value(value.clone()).map_err(DeError::custom)?,
            ));
        }

        if let Some(value) = raw.get("deviceCode") {
            return Ok(Self::DeviceCode(from_value(value.clone()).map_err(DeError::custom)?));
        }

        if let Some(value) = raw.get("implicit") {
            return Ok(Self::Implicit(from_value(value.clone()).map_err(DeError::custom)?));
        }

        if let Some(value) = raw.get("password") {
            return Ok(Self::Password(from_value(value.clone()).map_err(DeError::custom)?));
        }

        Err(DeError::custom("unknown OAuth flow variant"))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationCodeOAuthFlow {
    pub authorization_url: String,
    pub token_url: String,
    pub scopes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pkce_required: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClientCredentialsOAuthFlow {
    pub token_url: String,
    pub scopes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeOAuthFlow {
    pub device_authorization_url: String,
    pub token_url: String,
    pub scopes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImplicitOAuthFlow {
    pub authorization_url: String,
    pub scopes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasswordOAuthFlow {
    pub token_url: String,
    pub scopes: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentCardSignature {
    pub protected: String,
    pub signature: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<HashMap<String, Value>>,
}
