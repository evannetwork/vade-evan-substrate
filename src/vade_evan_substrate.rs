/*
  Copyright (c) 2018-present evan GmbH.

  Licensed under the Apache License, Version 2.0 (the "License");
  you may not use this file except in compliance with the License.
  You may obtain a copy of the License at

      http://www.apache.org/licenses/LICENSE-2.0

  Unless required by applicable law or agreed to in writing, software
  distributed under the License is distributed on an "AS IS" BASIS,
  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
  See the License for the specific language governing permissions and
  limitations under the License.
*/

extern crate regex;
extern crate vade;

use crate::signing::Signer;
use crate::utils::substrate::{
    add_payload_to_did,
    create_did,
    get_did,
    get_payload_count_for_did,
    is_whitelisted,
    update_payload_in_did,
    whitelist_identity,
};
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::error::Error;
use vade::{VadePlugin, VadePluginResultValue};

const EVAN_METHOD: &str = "did:evan";
const EVAN_METHOD_PREFIX: &str = "did:evan:";
const DID_SUBSTRATE: &str = "substrate";
const METHOD_REGEX: &str = r#"^(.*):0x(.*)$"#;

macro_rules! parse {
    ($data:expr, $type_name:expr) => {{
        serde_json::from_str($data)
            .map_err(|e| format!("{} when parsing {} {}", &e, $type_name, $data))?
    }};
}

macro_rules! ignore_unrelated {
    ($options:expr) => {{
        let type_options: TypeOptions = parse!($options, "options");
        match type_options.r#type.as_deref() {
            Some(DID_SUBSTRATE) => (),
            _ => return Ok(VadePluginResultValue::Ignored),
        };
    }};
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DidUpdateArguments {
    pub private_key: String,
    pub identity: String,
    pub operation: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityArguments {
    pub private_key: String,
    pub identity: String,
}

pub struct ResolverConfig {
    pub signer: Box<dyn Signer>,
    pub target: String,
}

/// Message passed to vade containing the desired did implementation.
/// Does not perform action if type does not indicate did type.
#[derive(Serialize, Deserialize)]
pub struct TypeOptions {
    pub r#type: Option<String>,
}

/// Resolver for DIDs on the Trust&Trace substrate chain
pub struct VadeEvanSubstrate {
    config: ResolverConfig,
}

impl VadeEvanSubstrate {
    /// Creates new instance of `VadeEvanSubstrate`.
    pub fn new(config: ResolverConfig) -> VadeEvanSubstrate {
        match env_logger::try_init() {
            Ok(_) | Err(_) => (),
        };
        VadeEvanSubstrate { config }
    }

    async fn set_did_document(
        &self,
        did: &str,
        private_key: &str,
        identity: &str,
        payload: &str,
    ) -> Result<Option<String>, Box<dyn Error>> {
        debug!(
            "setting DID document for did: {}, identity; {}",
            &did, &identity
        );
        let payload_count: u32 =
            get_payload_count_for_did(self.config.target.clone(), did.to_string()).await?;
        if payload_count > 0 {
            update_payload_in_did(
                self.config.target.clone(),
                0 as u32,
                payload.to_string(),
                did.to_string(),
                private_key.to_string(),
                &self.config.signer,
                hex::decode(identity)?,
            )
            .await?;
        } else {
            add_payload_to_did(
                self.config.target.clone(),
                payload.to_string(),
                did.to_string(),
                private_key.to_string(),
                &self.config.signer,
                hex::decode(identity)?,
            )
            .await?;
        }
        Ok(Some("".to_string()))
    }

    pub async fn is_whitelisted(
        &self,
        did: &str,
        private_key: &str,
    ) -> Result<bool, Box<dyn Error>> {
        let (_, substrate_identity) = convert_did_to_substrate_identity(&did)?;
        let substrate_identity_vec = hex::decode(&substrate_identity)?;
        let result = is_whitelisted(
            self.config.target.clone(),
            private_key.to_owned(),
            &self.config.signer,
            substrate_identity_vec,
        )
        .await?;
        Ok(result)
    }
}

#[async_trait(?Send)]
impl VadePlugin for VadeEvanSubstrate {
    /// Creates a new DID on substrate.
    ///
    /// # Arguments
    ///
    /// * `did_method` - did method to cater to, usually "did:evan"
    /// * `options` - serialized [`IdentityArguments`](https://docs.rs/vade_evan_substrate/*/vade_evan_substrate/vade_evan_substrate/struct.IdentityArguments.html)
    /// * `payload` - no payload required, so can be left empty
    ///
    async fn did_create(
        &mut self,
        did_method: &str,
        options: &str,
        payload: &str,
    ) -> Result<VadePluginResultValue<Option<String>>, Box<dyn Error>> {
        ignore_unrelated!(options);

        if !did_method.starts_with(EVAN_METHOD) {
            return Ok(VadePluginResultValue::Ignored);
        }
        let options: IdentityArguments = serde_json::from_str(&options)
            .map_err(|e| format!("{} when parsing {}", &e, &options))?;
        let (_, substrate_identity) = convert_did_to_substrate_identity(&options.identity)
            .map_err(|err| {
                format!(
                    "invalid identity in options: {}; {}",
                    &options.identity, &err
                )
            })?;
        let inner_result = create_did(
            self.config.target.clone(),
            options.private_key.clone(),
            &self.config.signer,
            hex::decode(&substrate_identity)?,
            match payload {
                "" => None,
                _ => Some(payload),
            },
        )
        .await?;

        Ok(VadePluginResultValue::Success(Some(format!(
            "\"{}:{}\"",
            &did_method, &inner_result
        ))))
    }

    /// Updates data related to a DID. Two updates are supported depending on the value of
    /// `options.operation`.
    ///
    /// - whitelistIdentity: whitelists identity `did` on substrate, this is required to be able to
    ///   perform transactions this this identity
    /// - setDidDocument: sets the DID document for `did`
    ///
    /// # Arguments
    ///
    /// * `did` - DID to update data for
    /// * `options` - serialized [`DidUpdateArguments`](https://docs.rs/vade_evan_substrate/*/vade_evan_substrate/vade_evan_substrate/struct.DidUpdateArguments.html)
    /// * `payload` - DID document to set or empty
    ///
    async fn did_update(
        &mut self,
        did: &str,
        options: &str,
        payload: &str,
    ) -> Result<VadePluginResultValue<Option<String>>, Box<dyn Error>> {
        ignore_unrelated!(options);
        
        if !did.starts_with(EVAN_METHOD_PREFIX) {
            return Ok(VadePluginResultValue::Ignored);
        }
        let input: DidUpdateArguments = serde_json::from_str(&options)
            .map_err(|e| format!("{} when parsing {}", &e, &options))?;
        let (method, substrate_identity) = convert_did_to_substrate_identity(&did)?;
        let substrate_identity_vec = hex::decode(&substrate_identity)?;

        match input.operation.as_str() {
            "ensureWhitelisted" => {
                // Check if identity is whitelisted
                let is_whitelisted = is_whitelisted(
                    self.config.target.clone(),
                    input.private_key.clone(),
                    &self.config.signer,
                    substrate_identity_vec,
                )
                .await?;
                // Execute whitelistIdentity operation
                if !is_whitelisted {
                    let mut new_input: DidUpdateArguments = serde_json::from_str(&options)?;
                    new_input.operation = "whitelistIdentity".to_owned();
                    Ok(self
                        .did_update(did, &serde_json::to_string(&new_input)?, payload)
                        .await?)
                } else {
                    Ok(VadePluginResultValue::Success(None))
                }
            }
            "whitelistIdentity" => {
                whitelist_identity(
                    self.config.target.clone(),
                    input.private_key.clone(),
                    &self.config.signer,
                    method,
                    substrate_identity_vec,
                )
                .await?;
                Ok(VadePluginResultValue::Success(None))
            }
            "setDidDocument" => {
                if !did.starts_with(EVAN_METHOD) {
                    return Ok(VadePluginResultValue::Ignored);
                }
                let (_, executing_did) = convert_did_to_substrate_identity(&input.identity)?;
                self.set_did_document(
                    &substrate_identity,
                    &input.private_key,
                    &executing_did,
                    payload,
                )
                .await?;
                Ok(VadePluginResultValue::Success(None))
            }
            _ => Err(Box::from(format!(
                "invalid did update operation \"{}\"",
                input.operation
            ))),
        }
    }

    /// Fetch data about a DID, which returns this DID's DID document.
    ///
    /// # Arguments
    ///
    /// * `did` - did to fetch data for
    async fn did_resolve(
        &mut self,
        did_id: &str,
    ) -> Result<VadePluginResultValue<Option<String>>, Box<dyn Error>> {
        if !did_id.starts_with(EVAN_METHOD) {
            return Ok(VadePluginResultValue::Ignored);
        }
        let identity_did = convert_did_to_substrate_identity(&did_id);
        if identity_did.is_err() {
            return Ok(VadePluginResultValue::Ignored);
        }
        let (_, substrate_identity) = identity_did?;
        let did_result = get_did(self.config.target.clone(), substrate_identity).await?;
        Ok(VadePluginResultValue::Success(Some(did_result)))
    }
}

/// Converts a DID to a substrate compatible method prefixed DID hex string.
///
/// # Arguments
///
/// `did` - a DID string, e.g. `did:evan:testcore:0x1234`
///
/// # Returns
///
/// tuple with
///     method of DID (e.g. 1 for core, 2 for testcore, 0 for unassigned)
///     32B substrate DID hex string without 0x prefix
fn convert_did_to_substrate_identity(did: &str) -> Result<(u8, String), Box<dyn Error>> {
    let re = Regex::new(METHOD_REGEX)?;
    let result = re.captures(&did);
    if let Some(caps) = result {
        match &caps[1] {
            "did:evan" => Ok((1, caps[2].to_string())),
            "did:evan:testcore" => Ok((2, caps[2].to_string())),
            "did:evan:zkp" => Ok((0, caps[2].to_string())),
            _ => Err(Box::from(format!("unknown DID format; {}", did))),
        }
    } else {
        Err(Box::from(format!("could not parse DID; {}", did)))
    }
}
