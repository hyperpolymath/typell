// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Diagnostic types for the TypeLL verification kernel.
//!
//! Error types are designed to be serializable so they can be transmitted
//! over the JSON-RPC bridge to PanLL for rendering.

use crate::types::{Dimension, Effect, Type, TypeVar, UsageQuantifier};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A source location span (byte offsets into source text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Span {
    /// Start byte offset.
    pub start: u32,
    /// End byte offset (exclusive).
    pub end: u32,
}

impl Span {
    /// Create a new span.
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// A zero-length span at the given offset.
    pub fn point(offset: u32) -> Self {
        Self { start: offset, end: offset }
    }

    /// A synthetic span for generated code (no source location).
    pub fn synthetic() -> Self {
        Self { start: 0, end: 0 }
    }
}

/// Result type alias for type-checking operations.
pub type TypeResult<T> = Result<T, TypeError>;

/// A type-checking error in the TypeLL kernel.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum TypeError {
    /// Two types could not be unified.
    #[error("type mismatch: expected {expected}, found {found}")]
    Mismatch {
        span: Span,
        expected: Type,
        found: Type,
        hint: Option<String>,
    },

    /// An undefined variable was referenced.
    #[error("undefined variable: {name}")]
    Undefined {
        span: Span,
        name: String,
        hint: Option<String>,
    },

    /// Two resource types have incompatible dimensions.
    #[error("dimension mismatch: {dim1} vs {dim2}")]
    DimensionMismatch {
        span: Span,
        dim1: Dimension,
        dim2: Dimension,
        hint: Option<String>,
    },

    /// Infinite type detected (occurs check failure).
    #[error("infinite type: {var} occurs in {ty}")]
    InfiniteType {
        span: Span,
        var: TypeVar,
        ty: Type,
    },

    /// A linearity constraint was violated.
    #[error("linearity violation: {message}")]
    LinearityViolation {
        span: Span,
        variable: String,
        expected_usage: UsageQuantifier,
        actual_usage: UsageQuantifier,
        message: String,
    },

    /// An effect was not declared in the type signature.
    #[error("undeclared effect: {effect}")]
    UndeclaredEffect {
        span: Span,
        effect: Effect,
        hint: Option<String>,
    },

    /// A refinement predicate is unsatisfiable.
    #[error("unsatisfiable refinement: {message}")]
    UnsatisfiableRefinement {
        span: Span,
        message: String,
    },

    /// A proof obligation could not be discharged.
    #[error("unresolved proof obligation: {obligation}")]
    UnresolvedProof {
        span: Span,
        obligation: String,
    },

    /// A session protocol was violated.
    #[error("session protocol violation: {message}")]
    SessionViolation {
        span: Span,
        message: String,
    },

    /// General custom error.
    #[error("{message}")]
    Custom {
        span: Span,
        message: String,
        hint: Option<String>,
    },
}

impl TypeError {
    /// Get the span of this error.
    pub fn span(&self) -> Span {
        match self {
            Self::Mismatch { span, .. }
            | Self::Undefined { span, .. }
            | Self::DimensionMismatch { span, .. }
            | Self::InfiniteType { span, .. }
            | Self::LinearityViolation { span, .. }
            | Self::UndeclaredEffect { span, .. }
            | Self::UnsatisfiableRefinement { span, .. }
            | Self::UnresolvedProof { span, .. }
            | Self::SessionViolation { span, .. }
            | Self::Custom { span, .. } => *span,
        }
    }

    /// Get the hint for this error, if any.
    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::Mismatch { hint, .. }
            | Self::Undefined { hint, .. }
            | Self::DimensionMismatch { hint, .. }
            | Self::UndeclaredEffect { hint, .. }
            | Self::Custom { hint, .. } => hint.as_deref(),
            _ => None,
        }
    }
}
