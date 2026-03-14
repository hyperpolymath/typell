// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Unification algorithm for the TypeLL kernel.
//!
//! Implements Robinson's unification with occurs check, extended for:
//! - Resource types with dimensional compatibility
//! - Effect row unification
//! - Session type duality checking
//!
//! Ported from Eclexia's `eclexia-typeck/src/unify.rs` and extended
//! to handle the full TypeLL type language.

use crate::error::{Span, TypeError, TypeResult};
use crate::types::{Effect, SessionType, Term, TermOp, Type, TypeVar};
use std::collections::HashMap;

/// Substitution mapping type variables to types.
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    map: HashMap<TypeVar, Type>,
}

impl Substitution {
    /// Create an empty substitution.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind a type variable to a type.
    pub fn bind(&mut self, var: TypeVar, ty: Type) {
        self.map.insert(var, ty);
    }

    /// Look up a type variable.
    pub fn lookup(&self, var: TypeVar) -> Option<&Type> {
        self.map.get(&var)
    }

    /// Apply this substitution to a type, following chains of variable bindings.
    pub fn apply(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => {
                if let Some(bound) = self.map.get(v) {
                    // Follow chains: if v -> t and t contains vars, apply again
                    self.apply(bound)
                } else {
                    ty.clone()
                }
            }
            Type::Primitive(_) | Type::Error | Type::Top | Type::Bottom => ty.clone(),
            Type::Named { name, args } => Type::Named {
                name: name.clone(),
                args: args.iter().map(|a| self.apply(a)).collect(),
            },
            Type::Function { params, ret, effects } => Type::Function {
                params: params.iter().map(|p| self.apply(p)).collect(),
                ret: Box::new(self.apply(ret)),
                effects: effects.clone(),
            },
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|e| self.apply(e)).collect())
            }
            Type::Array { elem, length } => Type::Array {
                elem: Box::new(self.apply(elem)),
                length: length.clone(),
            },
            Type::ForAll { vars, body } => Type::ForAll {
                vars: vars.clone(),
                body: Box::new(self.apply(body)),
            },
            Type::Resource { base, dimension } => Type::Resource {
                base: Box::new(self.apply(base)),
                dimension: *dimension,
            },
            Type::Refined { base, predicates } => Type::Refined {
                base: Box::new(self.apply(base)),
                predicates: predicates.clone(),
            },
            Type::Pi { param_name, param_type, body } => Type::Pi {
                param_name: param_name.clone(),
                param_type: Box::new(self.apply(param_type)),
                body: Box::new(self.apply(body)),
            },
            Type::Sigma { fst_name, fst_type, snd_type } => Type::Sigma {
                fst_name: fst_name.clone(),
                fst_type: Box::new(self.apply(fst_type)),
                snd_type: Box::new(self.apply(snd_type)),
            },
            Type::Session(s) => Type::Session(self.apply_session(s)),
        }
    }

    /// Apply this substitution to a session type, following chains
    /// for type variables embedded in Send/Recv positions.
    fn apply_session(&self, s: &SessionType) -> SessionType {
        match s {
            SessionType::Send(ty, cont) => SessionType::Send(
                Box::new(self.apply(ty)),
                Box::new(self.apply_session(cont)),
            ),
            SessionType::Recv(ty, cont) => SessionType::Recv(
                Box::new(self.apply(ty)),
                Box::new(self.apply_session(cont)),
            ),
            SessionType::Offer(branches) => SessionType::Offer(
                branches
                    .iter()
                    .map(|(l, s)| (l.clone(), self.apply_session(s)))
                    .collect(),
            ),
            SessionType::Select(branches) => SessionType::Select(
                branches
                    .iter()
                    .map(|(l, s)| (l.clone(), self.apply_session(s)))
                    .collect(),
            ),
            SessionType::End => SessionType::End,
            SessionType::Rec(v, body) => SessionType::Rec(
                v.clone(),
                Box::new(self.apply_session(body)),
            ),
            SessionType::RecVar(v) => SessionType::RecVar(v.clone()),
        }
    }

    /// Compose this substitution with another: `self ; other`.
    pub fn compose(&mut self, other: &Substitution) {
        // Apply `other` to all existing bindings
        for ty in self.map.values_mut() {
            *ty = other.apply(ty);
        }
        // Add bindings from `other` that we don't have
        for (var, ty) in &other.map {
            self.map.entry(*var).or_insert_with(|| ty.clone());
        }
    }
}

/// The unification engine.
pub struct Unifier {
    /// The current substitution.
    pub substitution: Substitution,
}

impl Unifier {
    /// Create a new unifier.
    pub fn new() -> Self {
        Self {
            substitution: Substitution::new(),
        }
    }

    /// Occurs check: does type variable `v` appear anywhere in type `t`?
    ///
    /// Prevents construction of infinite types like `a = List<a>`.
    fn occurs_check(&self, v: TypeVar, t: &Type) -> bool {
        let t = self.substitution.apply(t);
        match &t {
            Type::Var(v2) => v == *v2,
            Type::Named { args, .. } => args.iter().any(|a| self.occurs_check(v, a)),
            Type::Function { params, ret, .. } => {
                params.iter().any(|p| self.occurs_check(v, p))
                    || self.occurs_check(v, ret)
            }
            Type::Tuple(elems) => elems.iter().any(|e| self.occurs_check(v, e)),
            Type::Array { elem, .. } => self.occurs_check(v, elem),
            Type::ForAll { body, .. } => self.occurs_check(v, body),
            Type::Resource { base, .. } => self.occurs_check(v, base),
            Type::Refined { base, .. } => self.occurs_check(v, base),
            Type::Pi { param_type, body, .. } => {
                self.occurs_check(v, param_type) || self.occurs_check(v, body)
            }
            Type::Sigma { fst_type, snd_type, .. } => {
                self.occurs_check(v, fst_type) || self.occurs_check(v, snd_type)
            }
            _ => false,
        }
    }

    /// Unify two types, updating the internal substitution.
    ///
    /// Returns `Ok(())` on success, or a `TypeError` describing the mismatch.
    pub fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> TypeResult<()> {
        let t1 = self.substitution.apply(t1);
        let t2 = self.substitution.apply(t2);

        if t1 == t2 {
            return Ok(());
        }

        match (&t1, &t2) {
            // Error types unify with anything (error recovery).
            (Type::Error, _) | (_, Type::Error) => Ok(()),

            // Top unifies with anything — result is the other type.
            (Type::Top, other) | (other, Type::Top) => {
                // When Top meets a variable, bind the variable to the
                // concrete side (which is Top if both are Top, harmless).
                if let Type::Var(v) = other {
                    self.substitution.bind(*v, Type::Top);
                }
                Ok(())
            }

            // Bottom unifies with anything — result is Bottom.
            (Type::Bottom, other) | (other, Type::Bottom) => {
                if let Type::Var(v) = other {
                    self.substitution.bind(*v, Type::Bottom);
                }
                Ok(())
            }

            // Variable binding (with occurs check).
            (Type::Var(v), t) | (t, Type::Var(v)) => {
                if self.occurs_check(*v, t) {
                    return Err(TypeError::InfiniteType {
                        span,
                        var: *v,
                        ty: t.clone(),
                    });
                }
                self.substitution.bind(*v, t.clone());
                Ok(())
            }

            // Primitive types must match exactly, with numeric equivalences.
            (Type::Primitive(p1), Type::Primitive(p2)) => {
                use crate::types::PrimitiveType::*;
                if p1 == p2
                    || matches!(
                        (p1, p2),
                        (Float, F64) | (F64, Float) | (Int, I64) | (I64, Int)
                    )
                {
                    Ok(())
                } else {
                    Err(TypeError::Mismatch {
                        span,
                        expected: t1.clone(),
                        found: t2.clone(),
                        hint: None,
                    })
                }
            }

            // Function types: unify params pairwise, then return types.
            (
                Type::Function { params: p1, ret: r1, effects: e1 },
                Type::Function { params: p2, ret: r2, effects: e2 },
            ) => {
                if p1.len() != p2.len() {
                    return Err(TypeError::Mismatch {
                        span,
                        expected: t1.clone(),
                        found: t2.clone(),
                        hint: Some(format!(
                            "arity mismatch: expected {} parameters, found {}",
                            p1.len(), p2.len()
                        )),
                    });
                }
                for (a, b) in p1.iter().zip(p2.iter()) {
                    self.unify(a, b, span)?;
                }
                self.unify(r1, r2, span)?;
                self.unify_effects(e1, e2, span)?;
                Ok(())
            }

            // Tuple types: unify elements pairwise.
            (Type::Tuple(e1), Type::Tuple(e2)) => {
                if e1.len() != e2.len() {
                    return Err(TypeError::Mismatch {
                        span,
                        expected: t1.clone(),
                        found: t2.clone(),
                        hint: None,
                    });
                }
                for (a, b) in e1.iter().zip(e2.iter()) {
                    self.unify(a, b, span)?;
                }
                Ok(())
            }

            // Array types: unify element types and dependent lengths.
            (
                Type::Array { elem: e1, length: l1 },
                Type::Array { elem: e2, length: l2 },
            ) => {
                self.unify(e1, e2, span)?;
                // Unify dependent lengths when both are present
                match (l1, l2) {
                    (Some(len1), Some(len2)) => {
                        if !terms_unify(len1, len2) {
                            return Err(TypeError::Custom {
                                span,
                                message: format!(
                                    "array length mismatch: {} vs {}",
                                    len1, len2
                                ),
                                hint: Some(
                                    "dependent array lengths must be provably equal"
                                        .to_string(),
                                ),
                            });
                        }
                        Ok(())
                    }
                    // One or both lengths unknown — compatible
                    _ => Ok(()),
                }
            }

            // Named types: names must match, then unify args pairwise.
            (Type::Named { name: n1, args: a1 }, Type::Named { name: n2, args: a2 })
                if n1 == n2 =>
            {
                if a1.len() != a2.len() {
                    return Err(TypeError::Mismatch {
                        span,
                        expected: t1.clone(),
                        found: t2.clone(),
                        hint: None,
                    });
                }
                for (a, b) in a1.iter().zip(a2.iter()) {
                    self.unify(a, b, span)?;
                }
                Ok(())
            }

            // Resource types: dimensions must match, then unify base types.
            (
                Type::Resource { base: b1, dimension: d1 },
                Type::Resource { base: b2, dimension: d2 },
            ) => {
                if d1 != d2 {
                    return Err(TypeError::DimensionMismatch {
                        span,
                        dim1: *d1,
                        dim2: *d2,
                        hint: Some(
                            "resources must have compatible dimensions".to_string(),
                        ),
                    });
                }
                self.unify(b1, b2, span)
            }

            // Refined types: unify base types (predicate compatibility is
            // checked separately by the refinement checker).
            (Type::Refined { base: b1, .. }, Type::Refined { base: b2, .. }) => {
                self.unify(b1, b2, span)
            }

            // Pi types: unify parameter types, then body types.
            (
                Type::Pi { param_type: pt1, body: b1, .. },
                Type::Pi { param_type: pt2, body: b2, .. },
            ) => {
                self.unify(pt1, pt2, span)?;
                self.unify(b1, b2, span)
            }

            // Sigma types: unify both components.
            (
                Type::Sigma { fst_type: f1, snd_type: s1, .. },
                Type::Sigma { fst_type: f2, snd_type: s2, .. },
            ) => {
                self.unify(f1, f2, span)?;
                self.unify(s1, s2, span)
            }

            // Session types: unify structurally.
            (Type::Session(s1), Type::Session(s2)) => {
                self.unify_sessions(s1, s2, span)
            }

            // Everything else is a mismatch.
            _ => Err(TypeError::Mismatch {
                span,
                expected: t1.clone(),
                found: t2.clone(),
                hint: None,
            }),
        }
    }

    /// Unify effect rows using row-polymorphic semantics.
    ///
    /// Row polymorphism means an open effect row `{IO, ...rest}` can unify
    /// with a larger row `{IO, Network, Alloc}` by binding `rest` to
    /// `{Network, Alloc}`. Closed rows require set equality.
    fn unify_effects(
        &self,
        e1: &[Effect],
        e2: &[Effect],
        span: Span,
    ) -> TypeResult<()> {
        // Empty row on either side = effect-polymorphic (compatible with anything)
        if e1.is_empty() || e2.is_empty() {
            return Ok(());
        }

        // Exact match — fast path
        if e1 == e2 {
            return Ok(());
        }

        // Row-polymorphic check: each concrete effect in e1 must appear in e2,
        // and vice versa (for closed rows). Named effects act as row variables
        // when they start with a lowercase letter (convention: `r`, `rest`, etc.).
        let (concrete1, row_vars1) = partition_effects(e1);
        let (concrete2, row_vars2) = partition_effects(e2);

        // All concrete effects from the smaller set must be in the larger
        let missing_from_2: Vec<&&Effect> = concrete1
            .iter()
            .filter(|e| !concrete2.contains(e))
            .collect();
        let missing_from_1: Vec<&&Effect> = concrete2
            .iter()
            .filter(|e| !concrete1.contains(e))
            .collect();

        // If both sides have row variables, the unification succeeds —
        // the row variables absorb the difference
        if !row_vars1.is_empty() || !row_vars2.is_empty() {
            return Ok(());
        }

        // Closed rows: must be set-equal
        if missing_from_1.is_empty() && missing_from_2.is_empty() {
            Ok(())
        } else {
            let extra: Vec<String> = missing_from_2
                .iter()
                .chain(missing_from_1.iter())
                .map(|e| e.to_string())
                .collect();
            Err(TypeError::Custom {
                span,
                message: format!("effect row mismatch: unmatched effects [{}]", extra.join(", ")),
                hint: Some("add an effect row variable to make the function effect-polymorphic".to_string()),
            })
        }
    }

    /// Unify two session types, checking structural compatibility.
    ///
    /// Session types unify if they have the same structure with unifiable
    /// embedded types. This is NOT duality checking — duality is handled
    /// by `session::are_dual`. This is for checking that two types referring
    /// to the same session endpoint are compatible.
    fn unify_sessions(
        &mut self,
        s1: &SessionType,
        s2: &SessionType,
        span: Span,
    ) -> TypeResult<()> {
        match (s1, s2) {
            (SessionType::End, SessionType::End) => Ok(()),
            (SessionType::Send(t1, c1), SessionType::Send(t2, c2))
            | (SessionType::Recv(t1, c1), SessionType::Recv(t2, c2)) => {
                self.unify(t1, t2, span)?;
                self.unify_sessions(c1, c2, span)
            }
            (SessionType::Offer(b1), SessionType::Offer(b2))
            | (SessionType::Select(b1), SessionType::Select(b2)) => {
                if b1.len() != b2.len() {
                    return Err(TypeError::SessionViolation {
                        span,
                        message: format!(
                            "branch count mismatch: {} vs {}",
                            b1.len(),
                            b2.len()
                        ),
                    });
                }
                for ((l1, s1), (l2, s2)) in b1.iter().zip(b2.iter()) {
                    if l1 != l2 {
                        return Err(TypeError::SessionViolation {
                            span,
                            message: format!(
                                "branch label mismatch: '{}' vs '{}'",
                                l1, l2
                            ),
                        });
                    }
                    self.unify_sessions(s1, s2, span)?;
                }
                Ok(())
            }
            (SessionType::Rec(v1, b1), SessionType::Rec(v2, b2)) if v1 == v2 => {
                self.unify_sessions(b1, b2, span)
            }
            (SessionType::RecVar(v1), SessionType::RecVar(v2)) if v1 == v2 => Ok(()),
            _ => Err(TypeError::SessionViolation {
                span,
                message: format!("incompatible session types: {:?} vs {:?}", s1, s2),
            }),
        }
    }
}

/// Partition effects into concrete effects and row variables.
///
/// Convention: `Effect::Named` with a lowercase first character is treated
/// as a row variable (e.g., `r`, `rest`, `e`). This mirrors algebraic
/// effect systems in Koka, Frank, and Eff.
fn partition_effects(effects: &[Effect]) -> (Vec<&Effect>, Vec<&Effect>) {
    let mut concrete = Vec::new();
    let mut row_vars = Vec::new();
    for e in effects {
        match e {
            Effect::Named(name) if name.starts_with(|c: char| c.is_lowercase()) => {
                row_vars.push(e);
            }
            _ => concrete.push(e),
        }
    }
    (concrete, row_vars)
}

impl Default for Unifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Check whether two dependent type terms unify.
///
/// Term unification is used for dependent length checking (e.g.,
/// `Vec n a` unifies with `Vec (a + b) a` only if `n` and `a + b`
/// are provably equal). Variables unify with anything (they act as
/// unknowns). Literals must match exactly. Binary operations unify
/// if their operator and operands unify. Applications unify if
/// function names match and all arguments unify.
///
/// This is a conservative syntactic check — an SMT solver would be
/// needed for semantic equality (e.g., `a + b == b + a`).
pub fn terms_unify(t1: &Term, t2: &Term) -> bool {
    match (t1, t2) {
        // Variables unify with anything (they're unknowns)
        (Term::Var(_), _) | (_, Term::Var(_)) => true,
        // Literals must match exactly
        (Term::Lit(a), Term::Lit(b)) => a == b,
        // Binary ops: same operator + operands unify
        (
            Term::BinOp { op: op1, lhs: l1, rhs: r1 },
            Term::BinOp { op: op2, lhs: l2, rhs: r2 },
        ) => op1 == op2 && terms_unify(l1, l2) && terms_unify(r1, r2),
        // Applications: same function + args unify
        (
            Term::App { func: f1, args: a1 },
            Term::App { func: f2, args: a2 },
        ) => f1 == f2 && a1.len() == a2.len()
            && a1.iter().zip(a2.iter()).all(|(a, b)| terms_unify(a, b)),
        _ => false,
    }
}

/// Try to evaluate a Term to a compile-time integer constant.
///
/// Used for dimensional exponent evaluation and dependent length
/// resolution. Returns `None` if the term contains free variables
/// or cannot be evaluated statically.
pub fn eval_term_to_i64(term: &Term) -> Option<i64> {
    match term {
        Term::Lit(n) => Some(*n),
        Term::BinOp { op, lhs, rhs } => {
            let l = eval_term_to_i64(lhs)?;
            let r = eval_term_to_i64(rhs)?;
            match op {
                TermOp::Add => Some(l + r),
                TermOp::Sub => Some(l - r),
                TermOp::Mul => Some(l * r),
                TermOp::Div => {
                    if r == 0 { None } else { Some(l / r) }
                }
                TermOp::Mod => {
                    if r == 0 { None } else { Some(l % r) }
                }
            }
        }
        // Variables and applications cannot be evaluated statically
        Term::Var(_) | Term::App { .. } => None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Dimension, PrimitiveType};

    #[test]
    fn test_unify_identical_primitives() {
        let mut u = Unifier::new();
        let int = Type::Primitive(PrimitiveType::Int);
        assert!(u.unify(&int, &int, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_variable_with_concrete() {
        let mut u = Unifier::new();
        let var = Type::Var(TypeVar(0));
        let int = Type::Primitive(PrimitiveType::Int);
        assert!(u.unify(&var, &int, Span::synthetic()).is_ok());
        assert_eq!(u.substitution.apply(&var), int);
    }

    #[test]
    fn test_occurs_check_prevents_infinite_type() {
        let mut u = Unifier::new();
        let var = Type::Var(TypeVar(0));
        let list_of_var = Type::Named {
            name: "List".to_string(),
            args: vec![Type::Var(TypeVar(0))],
        };
        assert!(u.unify(&var, &list_of_var, Span::synthetic()).is_err());
    }

    #[test]
    fn test_unify_functions() {
        let mut u = Unifier::new();
        let f1 = Type::Function {
            params: vec![Type::Var(TypeVar(0))],
            ret: Box::new(Type::Var(TypeVar(0))),
            effects: vec![],
        };
        let f2 = Type::Function {
            params: vec![Type::Primitive(PrimitiveType::Int)],
            ret: Box::new(Type::Primitive(PrimitiveType::Int)),
            effects: vec![],
        };
        assert!(u.unify(&f1, &f2, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_resource_dimension_mismatch() {
        let mut u = Unifier::new();
        let r1 = Type::Resource {
            base: Box::new(Type::Primitive(PrimitiveType::Float)),
            dimension: Dimension::energy(),
        };
        let r2 = Type::Resource {
            base: Box::new(Type::Primitive(PrimitiveType::Float)),
            dimension: Dimension::time(),
        };
        let result = u.unify(&r1, &r2, Span::synthetic());
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TypeError::DimensionMismatch { .. }
        ));
    }

    #[test]
    fn test_unify_resource_same_dimension() {
        let mut u = Unifier::new();
        let r1 = Type::Resource {
            base: Box::new(Type::Primitive(PrimitiveType::Float)),
            dimension: Dimension::energy(),
        };
        let r2 = Type::Resource {
            base: Box::new(Type::Primitive(PrimitiveType::Float)),
            dimension: Dimension::energy(),
        };
        assert!(u.unify(&r1, &r2, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_substitution_composition() {
        let mut s1 = Substitution::new();
        s1.bind(TypeVar(0), Type::Var(TypeVar(1)));

        let mut s2 = Substitution::new();
        s2.bind(TypeVar(1), Type::Primitive(PrimitiveType::Int));

        s1.compose(&s2);
        assert_eq!(
            s1.apply(&Type::Var(TypeVar(0))),
            Type::Primitive(PrimitiveType::Int)
        );
    }

    // ========================================================================
    // Dependent length unification tests
    // ========================================================================

    #[test]
    fn test_unify_arrays_same_literal_length() {
        let mut u = Unifier::new();
        let a1 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: Some(Term::Lit(5)),
        };
        let a2 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: Some(Term::Lit(5)),
        };
        assert!(u.unify(&a1, &a2, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_arrays_different_literal_length() {
        let mut u = Unifier::new();
        let a1 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: Some(Term::Lit(5)),
        };
        let a2 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: Some(Term::Lit(10)),
        };
        assert!(u.unify(&a1, &a2, Span::synthetic()).is_err());
    }

    #[test]
    fn test_unify_arrays_variable_length_unifies() {
        let mut u = Unifier::new();
        let a1 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: Some(Term::Var("n".to_string())),
        };
        let a2 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: Some(Term::Lit(42)),
        };
        assert!(u.unify(&a1, &a2, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_arrays_one_unknown_length() {
        let mut u = Unifier::new();
        let a1 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: Some(Term::Lit(5)),
        };
        let a2 = Type::Array {
            elem: Box::new(Type::Primitive(PrimitiveType::Int)),
            length: None,
        };
        assert!(u.unify(&a1, &a2, Span::synthetic()).is_ok());
    }

    // ========================================================================
    // Session type unification tests
    // ========================================================================

    #[test]
    fn test_unify_session_send() {
        let mut u = Unifier::new();
        let s1 = Type::Session(SessionType::Send(
            Box::new(Type::Var(TypeVar(0))),
            Box::new(SessionType::End),
        ));
        let s2 = Type::Session(SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        ));
        assert!(u.unify(&s1, &s2, Span::synthetic()).is_ok());
        // The type variable should now be bound to Int
        assert_eq!(
            u.substitution.apply(&Type::Var(TypeVar(0))),
            Type::Primitive(PrimitiveType::Int)
        );
    }

    #[test]
    fn test_unify_session_mismatch() {
        let mut u = Unifier::new();
        let s1 = Type::Session(SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        ));
        let s2 = Type::Session(SessionType::Recv(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        ));
        assert!(u.unify(&s1, &s2, Span::synthetic()).is_err());
    }

    // ========================================================================
    // Effect row unification tests
    // ========================================================================

    #[test]
    fn test_unify_effects_closed_mismatch() {
        let u = Unifier::new();
        let e1 = vec![Effect::IO];
        let e2 = vec![Effect::IO, Effect::Network];
        // Closed rows with different effects should fail
        let result = u.unify_effects(&e1, &e2, Span::synthetic());
        assert!(result.is_err());
    }

    #[test]
    fn test_unify_effects_with_row_variable() {
        let u = Unifier::new();
        // row variable "rest" absorbs the difference
        let e1 = vec![Effect::IO, Effect::Named("rest".to_string())];
        let e2 = vec![Effect::IO, Effect::Network];
        assert!(u.unify_effects(&e1, &e2, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_effects_empty_is_polymorphic() {
        let u = Unifier::new();
        let e1: Vec<Effect> = vec![];
        let e2 = vec![Effect::IO, Effect::Network];
        assert!(u.unify_effects(&e1, &e2, Span::synthetic()).is_ok());
    }

    // ========================================================================
    // Term unification and evaluation tests
    // ========================================================================

    #[test]
    fn test_terms_unify_literals() {
        assert!(terms_unify(&Term::Lit(5), &Term::Lit(5)));
        assert!(!terms_unify(&Term::Lit(5), &Term::Lit(10)));
    }

    #[test]
    fn test_terms_unify_variable_with_anything() {
        assert!(terms_unify(
            &Term::Var("n".to_string()),
            &Term::Lit(42)
        ));
    }

    #[test]
    fn test_terms_unify_binop() {
        let t1 = Term::BinOp {
            op: TermOp::Add,
            lhs: Box::new(Term::Var("a".to_string())),
            rhs: Box::new(Term::Var("b".to_string())),
        };
        let t2 = Term::BinOp {
            op: TermOp::Add,
            lhs: Box::new(Term::Lit(3)),
            rhs: Box::new(Term::Lit(4)),
        };
        assert!(terms_unify(&t1, &t2));
    }

    #[test]
    fn test_eval_term_literal() {
        assert_eq!(eval_term_to_i64(&Term::Lit(42)), Some(42));
    }

    #[test]
    fn test_eval_term_arithmetic() {
        let term = Term::BinOp {
            op: TermOp::Mul,
            lhs: Box::new(Term::Lit(3)),
            rhs: Box::new(Term::Lit(4)),
        };
        assert_eq!(eval_term_to_i64(&term), Some(12));
    }

    #[test]
    fn test_eval_term_variable_unknown() {
        assert_eq!(eval_term_to_i64(&Term::Var("x".to_string())), None);
    }

    #[test]
    fn test_eval_term_div_by_zero() {
        let term = Term::BinOp {
            op: TermOp::Div,
            lhs: Box::new(Term::Lit(10)),
            rhs: Box::new(Term::Lit(0)),
        };
        assert_eq!(eval_term_to_i64(&term), None);
    }

    // ========================================================================
    // Substitution applies through session types
    // ========================================================================

    // ========================================================================
    // Top and Bottom unification tests
    // ========================================================================

    #[test]
    fn test_unify_top_with_concrete() {
        let mut u = Unifier::new();
        let top = Type::Top;
        let int = Type::Primitive(PrimitiveType::Int);
        assert!(u.unify(&top, &int, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_bottom_with_concrete() {
        let mut u = Unifier::new();
        let bottom = Type::Bottom;
        let int = Type::Primitive(PrimitiveType::Int);
        assert!(u.unify(&bottom, &int, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_top_with_variable() {
        let mut u = Unifier::new();
        let var = Type::Var(TypeVar(5));
        assert!(u.unify(&Type::Top, &var, Span::synthetic()).is_ok());
        assert_eq!(u.substitution.apply(&var), Type::Top);
    }

    #[test]
    fn test_unify_bottom_with_variable() {
        let mut u = Unifier::new();
        let var = Type::Var(TypeVar(6));
        assert!(u.unify(&Type::Bottom, &var, Span::synthetic()).is_ok());
        assert_eq!(u.substitution.apply(&var), Type::Bottom);
    }

    #[test]
    fn test_unify_top_with_top() {
        let mut u = Unifier::new();
        assert!(u.unify(&Type::Top, &Type::Top, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_unify_bottom_with_bottom() {
        let mut u = Unifier::new();
        assert!(u.unify(&Type::Bottom, &Type::Bottom, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_substitution_preserves_top_bottom() {
        let mut s = Substitution::new();
        s.bind(TypeVar(0), Type::Primitive(PrimitiveType::Int));
        assert_eq!(s.apply(&Type::Top), Type::Top);
        assert_eq!(s.apply(&Type::Bottom), Type::Bottom);
    }

    #[test]
    fn test_substitution_applies_through_session() {
        let mut s = Substitution::new();
        s.bind(TypeVar(0), Type::Primitive(PrimitiveType::String));

        let session_ty = Type::Session(SessionType::Send(
            Box::new(Type::Var(TypeVar(0))),
            Box::new(SessionType::End),
        ));

        let applied = s.apply(&session_ty);
        let expected = Type::Session(SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::String)),
            Box::new(SessionType::End),
        ));
        assert_eq!(applied, expected);
    }
}
