// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Session type protocol checking for the TypeLL kernel.
//!
//! Session types ensure that communicating parties follow compatible
//! protocols. The key operation is **duality**: if one side sends,
//! the other must receive, and vice versa.
//!
//! ## Example
//!
//! ```text
//! type BuyProtocol = Send<Item, Recv<Price, Select<{
//!     accept: Send<Payment, Recv<Receipt, End>>,
//!     reject: End,
//! }>>>
//!
//! type SellProtocol = dual(BuyProtocol)
//! // = Recv<Item, Send<Price, Offer<{
//! //     accept: Recv<Payment, Send<Receipt, End>>,
//! //     reject: End,
//! // }>>>
//! ```
//!
//! ## Current Status
//!
//! Foundation with duality computation and basic compatibility checking.
//! TODO: recursive session types, subtyping, multi-party sessions.

use crate::types::SessionType;

/// Compute the dual of a session type.
///
/// Duality swaps sends/receives and offers/selects, recursively.
/// This is the core operation that ensures two communicating parties
/// are compatible: if party A has session type S, party B must have
/// dual(S).
pub fn dual(session: &SessionType) -> SessionType {
    match session {
        SessionType::Send(ty, cont) => {
            SessionType::Recv(ty.clone(), Box::new(dual(cont)))
        }
        SessionType::Recv(ty, cont) => {
            SessionType::Send(ty.clone(), Box::new(dual(cont)))
        }
        SessionType::Offer(branches) => {
            SessionType::Select(
                branches
                    .iter()
                    .map(|(label, s)| (label.clone(), dual(s)))
                    .collect(),
            )
        }
        SessionType::Select(branches) => {
            SessionType::Offer(
                branches
                    .iter()
                    .map(|(label, s)| (label.clone(), dual(s)))
                    .collect(),
            )
        }
        SessionType::End => SessionType::End,
        SessionType::Rec(var, body) => {
            SessionType::Rec(var.clone(), Box::new(dual(body)))
        }
        SessionType::RecVar(var) => SessionType::RecVar(var.clone()),
    }
}

/// Check whether two session types are dual to each other.
pub fn are_dual(s1: &SessionType, s2: &SessionType) -> bool {
    let d1 = dual(s1);
    d1 == *s2
}

/// Check whether a session type is well-formed.
///
/// A session type is well-formed if:
/// - All recursive variables are bound by a `Rec` binder
/// - All branches in `Offer`/`Select` have distinct labels
/// - The protocol is contractive (no infinite unfolding without communication)
///
/// TODO: implement full well-formedness checking.
pub fn is_well_formed(session: &SessionType) -> bool {
    is_well_formed_inner(session, &[])
}

fn is_well_formed_inner(session: &SessionType, bound_vars: &[&str]) -> bool {
    match session {
        SessionType::End => true,
        SessionType::Send(_, cont) | SessionType::Recv(_, cont) => {
            is_well_formed_inner(cont, bound_vars)
        }
        SessionType::Offer(branches) | SessionType::Select(branches) => {
            // Check for duplicate labels
            let mut seen = std::collections::HashSet::new();
            for (label, s) in branches {
                if !seen.insert(label.as_str()) {
                    return false; // Duplicate label
                }
                if !is_well_formed_inner(s, bound_vars) {
                    return false;
                }
            }
            true
        }
        SessionType::Rec(var, body) => {
            let mut new_bound = bound_vars.to_vec();
            new_bound.push(var.as_str());
            is_well_formed_inner(body, &new_bound)
        }
        SessionType::RecVar(var) => {
            bound_vars.contains(&var.as_str())
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PrimitiveType, Type};

    #[test]
    fn test_dual_send_recv() {
        let send = SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        );
        let expected = SessionType::Recv(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        );
        assert_eq!(dual(&send), expected);
    }

    #[test]
    fn test_dual_is_involution() {
        let s = SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::String)),
            Box::new(SessionType::Recv(
                Box::new(Type::Primitive(PrimitiveType::Int)),
                Box::new(SessionType::End),
            )),
        );
        assert_eq!(dual(&dual(&s)), s);
    }

    #[test]
    fn test_are_dual() {
        let buyer = SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::String)),
            Box::new(SessionType::Recv(
                Box::new(Type::Primitive(PrimitiveType::Int)),
                Box::new(SessionType::End),
            )),
        );
        let seller = SessionType::Recv(
            Box::new(Type::Primitive(PrimitiveType::String)),
            Box::new(SessionType::Send(
                Box::new(Type::Primitive(PrimitiveType::Int)),
                Box::new(SessionType::End),
            )),
        );
        assert!(are_dual(&buyer, &seller));
    }

    #[test]
    fn test_well_formed_simple() {
        let s = SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        );
        assert!(is_well_formed(&s));
    }

    #[test]
    fn test_well_formed_unbound_var() {
        let s = SessionType::RecVar("x".to_string());
        assert!(!is_well_formed(&s));
    }

    #[test]
    fn test_well_formed_bound_var() {
        let s = SessionType::Rec(
            "x".to_string(),
            Box::new(SessionType::Send(
                Box::new(Type::Primitive(PrimitiveType::Int)),
                Box::new(SessionType::RecVar("x".to_string())),
            )),
        );
        assert!(is_well_formed(&s));
    }

    #[test]
    fn test_offer_duplicate_labels_not_well_formed() {
        let s = SessionType::Offer(vec![
            ("a".to_string(), SessionType::End),
            ("a".to_string(), SessionType::End),
        ]);
        assert!(!is_well_formed(&s));
    }
}
