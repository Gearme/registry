//! A module for client storage implementations.

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, pin::Pin, time::SystemTime};
use warg_crypto::{
    hash::{DynHash, HashAlgorithm},
    signing,
};
use warg_protocol::{
    package::{self, PackageRecord, PACKAGE_RECORD_VERSION},
    registry::{MapCheckpoint, RecordId},
    ProtoEnvelope, SerdeEnvelope, Version,
};

mod fs;
pub use fs::*;

/// Trait for package storage implementations.
///
/// Package storage data must be synchronized if shared between
/// multiple threads and processes.
#[async_trait]
pub trait PackageStorage: Send + Sync {
    /// Loads the package information for all packages in the storage.
    async fn load_packages(&self) -> Result<Vec<PackageInfo>>;

    /// Loads the package information from the storage.
    ///
    /// Returns `Ok(None)` if the information is not present.
    async fn load_package(&self, package: &str) -> Result<Option<PackageInfo>>;

    /// Stores the package information in the storage.
    async fn store_package(&self, info: &PackageInfo) -> Result<()>;

    /// Loads information about a pending publish operation.
    ///
    /// Returns `Ok(None)` if the information is not present.
    async fn load_publish(&self) -> Result<Option<PublishInfo>>;

    /// Stores information about a pending publish operation.
    ///
    /// If the info is `None`, the any existing publish information is deleted.
    async fn store_publish(&self, info: Option<&PublishInfo>) -> Result<()>;
}

/// Trait for content storage implementations.
///
/// Content storage data must be synchronized if shared between
/// multiple threads and processes.
#[async_trait]
pub trait ContentStorage: Send + Sync {
    /// Gets the location of the content associated with the given digest if it
    /// exists as a file on disk.
    ///
    /// Returns `None` if the content is not present on disk.
    fn content_location(&self, digest: &DynHash) -> Option<PathBuf>;

    /// Loads the content associated with the given digest as a stream.
    ///
    /// If the content is not found, `Ok(None)` is returned.
    async fn load_content(
        &self,
        digest: &DynHash,
    ) -> Result<Option<Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>>>;

    /// Stores the given stream as content.
    ///
    /// If `expected_digest` is `Some`, the storage will verify that the written
    /// content matches the given digest. If the digests do not match, an
    /// error is returned.
    ///
    /// Returns the hash of the written content.
    async fn store_content(
        &self,
        stream: Pin<Box<dyn Stream<Item = Result<Bytes>> + Send + Sync>>,
        expected_digest: Option<&DynHash>,
    ) -> Result<DynHash>;
}

/// Represents information about a registry package.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    /// The name of the package.
    pub name: String,
    /// The last known checkpoint of the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<SerdeEnvelope<MapCheckpoint>>,
    /// The current validation state of the package.
    #[serde(default)]
    pub state: package::Validator,
}

impl PackageInfo {
    /// Creates a new package info for the given package name and url.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            checkpoint: None,
            state: package::Validator::default(),
        }
    }
}

/// Represents a record entry being published.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PublishEntry {
    /// The package is being initialized.
    Init,
    /// A new release entry is being published.
    Release {
        /// The version of the release.
        version: Version,
        /// The content digest of the release.
        content: DynHash,
    },
}

/// Represents information about a package publish.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishInfo {
    /// The name of the package being published.
    pub package: String,
    /// The new record entries to publish.
    pub entries: Vec<PublishEntry>,
}

impl PublishInfo {
    /// Determines if the publish information is initializing the package.
    pub fn initializing(&self) -> bool {
        self.entries.iter().any(|e| matches!(e, PublishEntry::Init))
    }

    pub(crate) fn finalize(
        self,
        signing_key: &signing::PrivateKey,
        head: Option<RecordId>,
    ) -> Result<(ProtoEnvelope<PackageRecord>, Vec<DynHash>)> {
        let mut contents = Vec::new();
        let mut entries = Vec::with_capacity(self.entries.len());
        for entry in self.entries {
            match entry {
                PublishEntry::Init => {
                    entries.push(package::PackageEntry::Init {
                        hash_algorithm: HashAlgorithm::Sha256,
                        key: signing_key.public_key(),
                    });
                }
                PublishEntry::Release { version, content } => {
                    contents.push(content.clone());
                    entries.push(package::PackageEntry::Release { version, content });
                }
            }
        }

        let record = package::PackageRecord {
            prev: head,
            version: PACKAGE_RECORD_VERSION,
            // TODO: this seems wrong to record the current time client-side
            // How can we guarantee that the timestamps are monotonic?
            // Should incrementing timestamps even be a requirement?
            timestamp: SystemTime::now(),
            entries,
        };

        Ok((
            ProtoEnvelope::signed_contents(signing_key, record)?,
            contents,
        ))
    }
}
