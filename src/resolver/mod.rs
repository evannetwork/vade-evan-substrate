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

extern crate vade;
use reqwest;
use async_trait::async_trait;
use vade::traits::{ DidResolver };


/// Resolver for DIDs on evan.network (currently on testnet)
pub struct SubstrateDidResolverEvan {
}

impl SubstrateDidResolverEvan {
    /// Creates new instance of `SubstrateDidResolverEvan`.
    pub fn new() -> SubstrateDidResolverEvan {
        SubstrateDidResolverEvan { }
    }
}

#[async_trait(?Send)]
impl DidResolver for SubstrateDidResolverEvan {
    /// Checks given DID document.
    /// A DID document is considered as valid if returning ().
    /// Resolver may throw to indicate
    /// - that it is not responsible for this DID
    /// - that it considers this DID as invalid
    ///
    /// Currently the test `did_name` `"test"` is accepted as valid.
    ///
    /// # Arguments
    ///
    /// * `did_name` - did_name to check document for
    /// * `value` - value to check
    async fn check_did(&self, _did_name: &str, _value: &str) -> Result<(), Box<dyn std::error::Error>> {
        unimplemented!();
    }

    /// Gets document for given did name.
    ///
    /// # Arguments
    ///
    /// * `did_id` - did id to fetch
    async fn get_did_document(&self, did_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        call_substrate();
    }

    /// Sets document for given did name.
    ///
    /// # Arguments
    ///
    /// * `did_name` - did_name to set value for
    /// * `value` - value to set
    async fn set_did_document(&mut self, _did_id: &str, _value: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        unimplemented!();
    }
}