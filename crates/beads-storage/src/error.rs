//! Storage error types.

/// Errors that can occur during storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The requested entity was not found.
    #[error("{entity} not found: {id}")]
    NotFound {
        /// The kind of entity (e.g., "issue", "config").
        entity: String,
        /// The identifier that was looked up.
        id: String,
    },

    /// An issue is already claimed by another assignee.
    #[error("issue already claimed by {assignee}")]
    AlreadyClaimed {
        /// Current assignee who holds the claim.
        assignee: String,
    },

    /// The database has not been initialized.
    #[error("database not initialized: {reason}")]
    NotInitialized {
        /// Why the database is considered uninitialized.
        reason: String,
    },

    /// An issue ID does not match the configured prefix.
    #[error("issue {id} does not match configured prefix {prefix}")]
    PrefixMismatch {
        /// The issue ID.
        id: String,
        /// The expected prefix.
        prefix: String,
    },

    /// A validation constraint was violated.
    #[error("validation error: {message}")]
    Validation {
        /// Description of the validation failure.
        message: String,
    },

    /// Adding a dependency would create a cycle in the dependency graph.
    #[error("adding this dependency would create a cycle")]
    CycleDetected,

    /// The database is locked by another process.
    #[error("database locked: {0}")]
    DatabaseLocked(String),

    /// Failed to establish or maintain a database connection.
    #[error("connection error: {0}")]
    Connection(String),

    /// A transaction operation failed.
    #[error("transaction error: {0}")]
    Transaction(String),

    /// A schema migration failed.
    #[error("migration {name} failed: {reason}")]
    Migration {
        /// Name of the migration that failed.
        name: String,
        /// Underlying error description.
        reason: String,
    },

    /// A raw SQLite query error.
    #[error("query error: {0}")]
    Query(#[from] rusqlite::Error),

    /// JSON serialization/deserialization failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Catch-all for unexpected internal errors.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Convenience alias used throughout the storage crate.
pub type Result<T> = std::result::Result<T, StorageError>;

impl StorageError {
    // -- Constructors --------------------------------------------------------

    /// Creates a [`StorageError::NotFound`] for the given entity kind and id.
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        }
    }

    /// Creates a [`StorageError::Validation`] with the given message.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    // -- Predicates ----------------------------------------------------------

    /// Returns `true` if this is a [`StorageError::NotFound`].
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }

    /// Returns `true` if the error is transient and the operation may succeed
    /// on retry (e.g., database locked, connection errors).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::DatabaseLocked(_) | Self::Connection(_) | Self::Transaction(_)
        )
    }
}
