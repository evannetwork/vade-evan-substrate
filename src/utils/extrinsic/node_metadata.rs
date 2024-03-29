// Copyright 2019 Parity Technologies (UK) Ltd. and Supercomputing Systems AG
// This file is part of substrate-subxt.
//
// subxt is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// subxt is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with substrate-subxt.  If not, see <http://www.gnu.org/licenses/>.

use super::frame_metadata::{
    DecodeDifferent,
    RuntimeMetadata,
    RuntimeMetadataPrefixed,
    StorageEntryModifier,
    StorageEntryType,
    StorageHasher,
    META_RESERVED,
};
use crate::utils::substrate;
use parity_scale_codec::{Decode, Encode};
use sp_storage::StorageKey;
use std::{collections::HashMap, marker::PhantomData, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error("Error converting substrate metadata: {0}")]
    Conversion(#[from] ConversionError),
    #[error("Module not found")]
    ModuleNotFound(String),
    #[error("Module with events not found")]
    ModuleWithEventsNotFound(u8),
    #[error("Event not found")]
    EventNotFound(u8),
    #[error("Storage not found")]
    StorageNotFound(&'static str),
    #[error("Storage type error")]
    StorageTypeError,
    #[error("Map value type error")]
    MapValueTypeError,
}

#[derive(Clone, Debug)]
pub struct Metadata {
    modules: HashMap<String, ModuleMetadata>,
    modules_with_calls: HashMap<String, ModuleWithCalls>,
    modules_with_events: HashMap<String, ModuleWithEvents>,
    modules_with_errors: HashMap<String, ModuleWithEvents>,
}

impl Metadata {
    pub fn module<S>(&self, name: S) -> Result<&ModuleMetadata, MetadataError>
    where
        S: ToString,
    {
        let name = name.to_string();
        self.modules
            .get(&name)
            .ok_or(MetadataError::ModuleNotFound(name))
    }

    pub fn module_with_calls<S>(&self, name: S) -> Result<&ModuleWithCalls, MetadataError>
    where
        S: ToString,
    {
        let name = name.to_string();
        self.modules_with_calls
            .get(&name)
            .ok_or(MetadataError::ModuleNotFound(name))
    }

    pub fn module_with_events(&self, module_index: u8) -> Result<&ModuleWithEvents, MetadataError> {
        self.modules_with_events
            .values()
            .find(|&module| module.index == module_index)
            .ok_or(MetadataError::ModuleWithEventsNotFound(module_index))
    }

    pub fn module_with_errors(&self, module_index: u8) -> Result<&ModuleWithEvents, MetadataError> {
        self.modules_with_errors
            .values()
            .find(|&module| module.index == module_index)
            .ok_or(MetadataError::ModuleWithEventsNotFound(module_index))
    }

    pub fn parse(metadata: RuntimeMetadataPrefixed) -> Result<Self, MetadataError> {
        if metadata.0 != META_RESERVED {
            return Err(ConversionError::InvalidPrefix.into());
        }
        let meta = match metadata.1 {
            RuntimeMetadata::V11(meta) => meta,
            _ => return Err(ConversionError::InvalidVersion.into()),
        };
        let mut modules = HashMap::new();
        let mut modules_with_calls = HashMap::new();
        let mut modules_with_events = HashMap::new();
        let mut modules_with_errors = HashMap::new();
        for module in convert(meta.modules)?.into_iter() {
            let module_name = convert(module.name.clone())?;

            let mut storage_map = HashMap::new();
            if let Some(storage) = module.storage {
                let storage = convert(storage)?;
                let module_prefix = convert(storage.prefix)?;
                for entry in convert(storage.entries)?.into_iter() {
                    let storage_prefix = convert(entry.name.clone())?;
                    let entry = convert_entry(
                        module_prefix.clone().to_string(),
                        storage_prefix.clone().to_string(),
                        entry,
                    )?;
                    storage_map.insert(storage_prefix.to_string(), entry);
                }
            }
            modules.insert(
                module_name.clone().to_string(),
                ModuleMetadata {
                    name: module_name.clone().to_string(),
                    storage: storage_map,
                },
            );

            if let Some(calls) = module.calls {
                let mut call_map = HashMap::new();
                for (index, call) in convert(calls)?.into_iter().enumerate() {
                    let name = convert(call.name)?;
                    call_map.insert(name.to_string(), index as u8);
                }
                modules_with_calls.insert(
                    module_name.clone().to_string(),
                    ModuleWithCalls {
                        index: modules_with_calls.len() as u8,
                        name: module_name.clone().to_string(),
                        calls: call_map,
                    },
                );
            }
            if let Some(events) = module.event {
                let mut event_map = HashMap::new();
                for (index, event) in convert(events)?.into_iter().enumerate() {
                    event_map.insert(index as u8, convert_event(event)?);
                }
                modules_with_events.insert(
                    module_name.clone().to_string(),
                    ModuleWithEvents {
                        index: modules_with_events.len() as u8,
                        name: module_name.clone().to_string(),
                        events: event_map,
                    },
                );
            }

            let mut error_map = HashMap::new();
            for (index, event) in convert(module.errors)?.into_iter().enumerate() {
                error_map.insert(index as u8, convert_error(event)?);
            }
            modules_with_errors.insert(
                module_name.clone().to_string(),
                ModuleWithEvents {
                    index: modules_with_errors.len() as u8,
                    name: module_name.clone().to_string(),
                    events: error_map,
                },
            );
        }
        Ok(Metadata {
            modules,
            modules_with_calls,
            modules_with_events,
            modules_with_errors,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ModuleMetadata {
    name: String,
    storage: HashMap<String, StorageMetadata>,
    // constants
}

impl ModuleMetadata {
    pub fn storage(&self, key: &'static str) -> Result<&StorageMetadata, MetadataError> {
        self.storage
            .get(key)
            .ok_or(MetadataError::StorageNotFound(key))
    }
}

// Todo make nice list of Call args to facilitate call arg lookup
#[derive(Clone, Debug)]
pub struct ModuleWithCalls {
    pub index: u8,
    pub name: String,
    pub calls: HashMap<String, u8>,
}

#[derive(Clone, Debug)]
pub struct ModuleWithEvents {
    index: u8,
    name: String,
    events: HashMap<u8, ModuleEventMetadata>,
}

impl ModuleWithEvents {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn event(&self, index: u8) -> Result<&ModuleEventMetadata, MetadataError> {
        self.events
            .get(&index)
            .ok_or(MetadataError::EventNotFound(index))
    }
}

#[derive(Clone, Debug)]
pub struct StorageMetadata {
    module_prefix: String,
    storage_prefix: String,
    modifier: StorageEntryModifier,
    ty: StorageEntryType,
    default: Vec<u8>,
}

impl StorageMetadata {
    pub fn get_map<K: Encode, V: Decode + Clone>(&self) -> Result<StorageMap<K, V>, MetadataError> {
        match &self.ty {
            StorageEntryType::Map { hasher, .. } => {
                let module_prefix = self.module_prefix.as_bytes().to_vec();
                let storage_prefix = self.storage_prefix.as_bytes().to_vec();
                let hasher = hasher.to_owned();
                let default = Decode::decode(&mut &self.default[..])
                    .map_err(|_| MetadataError::MapValueTypeError)?;

                debug!(
                    "map for '{}' '{}' has hasher {:?}",
                    self.module_prefix, self.storage_prefix, hasher
                );
                Ok(StorageMap {
                    _marker: PhantomData,
                    module_prefix,
                    storage_prefix,
                    hasher,
                    default,
                })
            }
            _ => Err(MetadataError::StorageTypeError),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StorageValue {
    module_prefix: Vec<u8>,
    storage_prefix: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct StorageMap<K, V> {
    _marker: PhantomData<K>,
    module_prefix: Vec<u8>,
    storage_prefix: Vec<u8>,
    hasher: StorageHasher,
    default: V,
}

impl<K: Encode, V: Decode + Clone> StorageMap<K, V> {
    pub fn key(&self, key: K) -> StorageKey {
        let mut bytes = substrate::twox_128(&self.module_prefix).to_vec();
        bytes.extend(&substrate::twox_128(&self.storage_prefix)[..]);
        bytes.extend(key_hash(&key, &self.hasher));
        StorageKey(bytes)
    }
}

#[derive(Clone, Debug)]
pub struct StorageDoubleMap<K, Q, V> {
    _marker: PhantomData<K>,
    _marker2: PhantomData<Q>,
    module_prefix: Vec<u8>,
    storage_prefix: Vec<u8>,
    hasher: StorageHasher,
    key2_hasher: StorageHasher,
    default: V,
}

#[derive(Clone, Debug)]
pub struct ModuleEventMetadata {
    pub name: String,
    arguments: Vec<EventArg>,
}

impl ModuleEventMetadata {
    pub fn arguments(&self) -> Vec<EventArg> {
        self.arguments.to_vec()
    }
}

/// Naive representation of event argument types, supports current set of substrate EventArg types.
/// If and when Substrate uses `type-metadata`, this can be replaced.
///
/// Used to calculate the size of a instance of an event variant without having the concrete type,
/// so the raw bytes can be extracted from the encoded `Vec<EventRecord<E>>` (without `E` defined).
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum EventArg {
    Primitive(String),
    Vec(Box<EventArg>),
    Tuple(Vec<EventArg>),
}

impl FromStr for EventArg {
    type Err = ConversionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("Vec<") {
            if s.ends_with('>') {
                Ok(EventArg::Vec(Box::new(s[4..s.len() - 1].parse()?)))
            } else {
                Err(ConversionError::InvalidEventArg(
                    s.to_string(),
                    "Expected closing `>` for `Vec`",
                ))
            }
        } else if s.starts_with('(') {
            if s.ends_with(')') {
                let mut args = Vec::new();
                for arg in s[1..s.len() - 1].split(',') {
                    let arg = arg.trim().parse()?;
                    args.push(arg)
                }
                Ok(EventArg::Tuple(args))
            } else {
                Err(ConversionError::InvalidEventArg(
                    s.to_string(),
                    "Expecting closing `)` for tuple",
                ))
            }
        } else {
            Ok(EventArg::Primitive(s.to_string()))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Invalid prefix")]
    InvalidPrefix,
    #[error("Invalid version")]
    InvalidVersion,
    #[error("Expected DecodeDifferent::Decoded")]
    ExpectedDecoded,
    #[error("Invalid event arg {0}")]
    InvalidEventArg(String, &'static str),
}

fn convert<B: 'static, O: 'static>(dd: DecodeDifferent<B, O>) -> Result<O, ConversionError> {
    match dd {
        DecodeDifferent::Decoded(value) => Ok(value),
        _ => Err(ConversionError::ExpectedDecoded),
    }
}

fn convert_event(
    event: super::frame_metadata::EventMetadata,
) -> Result<ModuleEventMetadata, ConversionError> {
    let name = convert(event.name)?;
    let mut arguments = Vec::new();
    for arg in convert(event.arguments)? {
        let arg = arg.parse::<EventArg>()?;
        arguments.push(arg);
    }
    Ok(ModuleEventMetadata { name, arguments })
}

fn convert_error(
    event: super::frame_metadata::ErrorMetadata,
) -> Result<ModuleEventMetadata, ConversionError> {
    let name = convert(event.name)?;
    let arguments = Vec::new();
    Ok(ModuleEventMetadata { name, arguments })
}

fn convert_entry(
    module_prefix: String,
    storage_prefix: String,
    entry: super::frame_metadata::StorageEntryMetadata,
) -> Result<StorageMetadata, ConversionError> {
    let default = convert(entry.default)?;
    Ok(StorageMetadata {
        module_prefix,
        storage_prefix,
        modifier: entry.modifier,
        ty: entry.ty,
        default,
    })
}

/// generates the key's hash depending on the StorageHasher selected
fn key_hash<K: Encode>(key: &K, hasher: &StorageHasher) -> Vec<u8> {
    let encoded_key = key.encode();
    match hasher {
        StorageHasher::Identity => encoded_key.to_vec(),
        StorageHasher::Blake2_128 => substrate::blake2_128(&encoded_key).to_vec(),
        StorageHasher::Blake2_128Concat => {
            // copied from substrate Blake2_128Concat::hash since StorageHasher is not public
            let x: &[u8] = encoded_key.as_slice();
            substrate::blake2_128(x)
                .iter()
                .chain(x.iter())
                .cloned()
                .collect::<Vec<_>>()
        }
        StorageHasher::Blake2_256 => substrate::blake2_256(&encoded_key).to_vec(),
        StorageHasher::Twox128 => substrate::twox_128(&encoded_key).to_vec(),
        StorageHasher::Twox256 => substrate::twox_256(&encoded_key).to_vec(),
        StorageHasher::Twox64Concat => substrate::twox_64(&encoded_key).to_vec(),
    }
}
