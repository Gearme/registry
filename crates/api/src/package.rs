//! Types relating to the package API.

use crate::{content::ContentSource, FromError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use warg_protocol::{
    registry::{LogId, MapCheckpoint, RecordId},
    ProtoEnvelopeBody, SerdeEnvelope,
};

/// Represents a request to publish a package.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct PublishRequest {
    /// The name of the package being published.
    pub name: String,
    /// The publish record to add to the package log.
    pub record: ProtoEnvelopeBody,
    /// The content sources for the record.
    pub content_sources: Vec<ContentSource>,
}

/// Represents a pending record response.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "state", rename = "camelCase")]
pub enum PendingRecordResponse {
    /// The record has been published.
    Published {
        /// The URL of the published record.
        record_url: String,
    },
    /// The record has been rejected.
    Rejected {
        /// The reason the record was rejected.
        reason: String,
    },
    /// The record is still being processed.
    Processing {
        /// The URL of the publishing status.
        status_url: String,
    },
}

/// Represents a response to a record request.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RecordResponse {
    /// The body of the record.
    pub record: ProtoEnvelopeBody,
    /// The content sources of the record.
    pub content_sources: Vec<ContentSource>,
    /// The checkpoint of the record.
    pub checkpoint: SerdeEnvelope<MapCheckpoint>,
}

/// Represents an error from the package API.
#[non_exhaustive]
#[derive(Debug, Error, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PackageError {
    /// The provided package id was invalid.
    #[error("invalid package id: {message}")]
    InvalidPackageId {
        /// The validation error message.
        message: String,
    },
    /// The provided record id was invalid.
    #[error("invalid record id: {message}")]
    InvalidRecordId {
        /// The validation error message.
        message: String,
    },
    /// The provided record was invalid.
    #[error("invalid record: {message}")]
    InvalidRecord {
        /// The validation error message.
        message: String,
    },
    /// The provided package was not found.
    #[error("package log `{log_id}` was not found")]
    PackageIdNotFound {
        /// The id of the missing package log.
        log_id: LogId,
    },
    /// The provided package was not found.
    #[error("package `{name}` was not found")]
    PackageNotFound {
        /// The name of the missing package log.
        name: String,
    },
    /// The provided package record was not found.
    #[error("package record `{id}` was not found")]
    PackageRecordNotFound {
        /// The id of the missing package record.
        id: RecordId,
    },
    /// Failed to fetch from the content source.
    #[error("failed to fetch content: {message}")]
    FailedToFetchContent {
        /// The error message.
        message: String,
    },
    /// An error response was returned from the content source.
    #[error("cannot validate content source: {status_code} status returned from server")]
    ContentFetchErrorResponse {
        /// The error status code.
        status_code: u16,
    },
    /// The provided content source is not from the current host.
    #[error("content source `{url}` is not from the current host")]
    ContentUrlInvalid {
        /// The provided content source url.
        url: String,
    },
    /// An error occurred while performing the requested operation.
    #[error("an error occurred while performing the requested operation")]
    Operation,
    /// An error with a message occurred.
    #[error("{message}")]
    Message {
        /// The error message.
        message: String,
    },
}

impl From<String> for PackageError {
    fn from(message: String) -> Self {
        Self::Message { message }
    }
}

impl FromError for PackageError {
    fn from_error<E: std::error::Error>(error: E) -> Self {
        Self::from(error.to_string())
    }
}

/// Represents the result of a package API operation.
pub type PackageResult<T> = Result<T, PackageError>;
