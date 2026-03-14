// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bidirectional type inference for the TypeLL kernel.
//!
//! Implements a bidirectional type checking algorithm combining:
//! - Hindley-Milner inference (generalization + instantiation)
//! - Checking mode (push expected type down)
//! - Synthesis mode (pull inferred type up)
//!
//! Ported from Eclexia's `eclexia-typeck/src/infer.rs` and extended
//! for the full TypeLL type language.

use crate::error::{Span, TypeResult};
use crate::types::{Type, TypeScheme, TypeVar, UnifiedType};
use crate::unify::Substitution;
use std::collections::{HashMap, HashSet};

/// Type inference context — tracks variable bindings and generates fresh
/// type variables.
pub struct InferCtx {
    /// Variable bindings (name -> type scheme).
    bindings: HashMap<String, TypeScheme>,
    /// Parent scope for lexical scoping.
    parent: Option<Box<InferCtx>>,
    /// Next fresh type variable ID.
    next_var: u32,
}

impl InferCtx {
    /// Create a new empty inference context.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            parent: None,
            next_var: 0,
        }
    }

    /// Create a child scope.
    pub fn child(&self) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(Box::new(self.clone())),
            next_var: self.next_var,
        }
    }

    /// Generate a fresh type variable.
    pub fn fresh_var(&mut self) -> Type {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        Type::Var(var)
    }

    /// Insert a monomorphic binding.
    pub fn insert(&mut self, name: String, ty: UnifiedType) {
        self.bindings.insert(name, TypeScheme::mono(ty));
    }

    /// Insert a polymorphic binding.
    pub fn insert_scheme(&mut self, name: String, scheme: TypeScheme) {
        self.bindings.insert(name, scheme);
    }

    /// Look up a binding in this scope or parents.
    pub fn lookup(&self, name: &str) -> Option<&TypeScheme> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    /// Get all names available in scope (for error suggestions).
    pub fn available_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.bindings.keys().cloned().collect();
        if let Some(parent) = &self.parent {
            names.extend(parent.available_names());
        }
        names
    }

    // ========================================================================
    // Generalization and Instantiation
    // ========================================================================

    /// Generalize a type by abstracting free type variables into a ForAll.
    ///
    /// Variables that are free in the type but not free in the environment
    /// are universally quantified. This enables let-polymorphism:
    /// `let id = fn(x) { x }` gets type `forall a. a -> a`.
    pub fn generalize(&self, ty: &Type, subst: &Substitution) -> TypeScheme {
        let ty = subst.apply(ty);
        let free_in_ty = self.free_vars_in_type(&ty);
        let free_in_env = self.free_vars_in_env(subst);

        let generalizable: Vec<String> = free_in_ty
            .difference(&free_in_env)
            .map(|v| format!("t{}", v.0))
            .collect();

        if generalizable.is_empty() {
            TypeScheme::mono(UnifiedType::simple(ty))
        } else {
            TypeScheme {
                vars: generalizable,
                body: UnifiedType::simple(ty),
            }
        }
    }

    /// Instantiate a polymorphic type scheme with fresh type variables.
    ///
    /// Replaces each universally quantified variable with a fresh
    /// unification variable, enabling separate uses of a polymorphic
    /// binding to get independent types.
    pub fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        if scheme.vars.is_empty() {
            return scheme.body.base.clone();
        }

        let mut var_map = HashMap::new();
        for var_name in &scheme.vars {
            let fresh = self.fresh_var();
            var_map.insert(var_name.clone(), fresh);
        }

        self.substitute_named_vars(&scheme.body.base, &var_map)
    }

    // ========================================================================
    // Bidirectional type checking
    // ========================================================================

    /// Synthesis mode: infer the type of an expression.
    ///
    /// In synthesis mode, the type checker produces a type from the
    /// expression structure alone, without an expected type.
    ///
    /// This is a framework method — concrete expression types are handled
    /// by language-specific bridges (e.g., `typell-eclexia`).
    pub fn synthesize_var(&mut self, name: &str, span: Span) -> TypeResult<Type> {
        match self.lookup(name) {
            Some(scheme) => {
                let scheme = scheme.clone();
                Ok(self.instantiate(&scheme))
            }
            None => Err(crate::error::TypeError::Undefined {
                span,
                name: name.to_string(),
                hint: find_closest_match(name, &self.available_names()),
            }),
        }
    }

    /// Checking mode: check that an expression has the expected type.
    ///
    /// In checking mode, the expected type is pushed down into the
    /// expression, potentially guiding inference decisions. This is
    /// used when a type annotation or context provides the expected type.
    ///
    /// Returns `Ok(())` if the expression is well-typed at the expected type.
    pub fn check_against(
        &mut self,
        inferred: &Type,
        expected: &Type,
        span: Span,
        unifier: &mut crate::unify::Unifier,
    ) -> TypeResult<()> {
        unifier.unify(inferred, expected, span)
    }

    // ========================================================================
    // Free variable computation
    // ========================================================================

    /// Compute the set of free type variables in a type.
    fn free_vars_in_type(&self, ty: &Type) -> HashSet<TypeVar> {
        match ty {
            Type::Var(v) => {
                let mut set = HashSet::new();
                set.insert(*v);
                set
            }
            Type::Primitive(_) | Type::Error | Type::Top | Type::Bottom => HashSet::new(),
            Type::Named { args, .. } => {
                args.iter().flat_map(|a| self.free_vars_in_type(a)).collect()
            }
            Type::Function { params, ret, .. } => {
                let mut vars: HashSet<TypeVar> =
                    params.iter().flat_map(|p| self.free_vars_in_type(p)).collect();
                vars.extend(self.free_vars_in_type(ret));
                vars
            }
            Type::Tuple(elems) => {
                elems.iter().flat_map(|e| self.free_vars_in_type(e)).collect()
            }
            Type::Array { elem, .. } => self.free_vars_in_type(elem),
            Type::ForAll { vars, body } => {
                let mut free = self.free_vars_in_type(body);
                // Remove bound variables
                free.retain(|v| !vars.contains(&format!("t{}", v.0)));
                free
            }
            Type::Resource { base, .. } => self.free_vars_in_type(base),
            Type::Refined { base, .. } => self.free_vars_in_type(base),
            Type::Pi { param_type, body, .. } => {
                let mut vars = self.free_vars_in_type(param_type);
                vars.extend(self.free_vars_in_type(body));
                vars
            }
            Type::Sigma { fst_type, snd_type, .. } => {
                let mut vars = self.free_vars_in_type(fst_type);
                vars.extend(self.free_vars_in_type(snd_type));
                vars
            }
            Type::Session(s) => self.free_vars_in_session(s),
        }
    }

    /// Compute free type variables in a session type.
    fn free_vars_in_session(&self, s: &crate::types::SessionType) -> HashSet<TypeVar> {
        match s {
            crate::types::SessionType::Send(ty, cont)
            | crate::types::SessionType::Recv(ty, cont) => {
                let mut vars = self.free_vars_in_type(ty);
                vars.extend(self.free_vars_in_session(cont));
                vars
            }
            crate::types::SessionType::Offer(branches)
            | crate::types::SessionType::Select(branches) => {
                branches.iter().flat_map(|(_, s)| self.free_vars_in_session(s)).collect()
            }
            crate::types::SessionType::End
            | crate::types::SessionType::RecVar(_) => HashSet::new(),
            crate::types::SessionType::Rec(_, body) => self.free_vars_in_session(body),
        }
    }

    /// Compute the set of free type variables in the environment.
    fn free_vars_in_env(&self, subst: &Substitution) -> HashSet<TypeVar> {
        let mut vars = HashSet::new();
        for scheme in self.bindings.values() {
            let applied = subst.apply(&scheme.body.base);
            let free = self.free_vars_in_type(&applied);
            // Subtract the scheme's quantified variables
            for v in free {
                if !scheme.vars.contains(&format!("t{}", v.0)) {
                    vars.insert(v);
                }
            }
        }
        if let Some(parent) = &self.parent {
            vars.extend(parent.free_vars_in_env(subst));
        }
        vars
    }

    /// Substitute named type variables in a type.
    fn substitute_named_vars(&self, ty: &Type, var_map: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Var(v) => {
                let name = format!("t{}", v.0);
                var_map.get(&name).cloned().unwrap_or_else(|| ty.clone())
            }
            Type::Primitive(_) | Type::Error | Type::Top | Type::Bottom => ty.clone(),
            Type::Named { name, args } => Type::Named {
                name: name.clone(),
                args: args.iter().map(|a| self.substitute_named_vars(a, var_map)).collect(),
            },
            Type::Function { params, ret, effects } => Type::Function {
                params: params.iter().map(|p| self.substitute_named_vars(p, var_map)).collect(),
                ret: Box::new(self.substitute_named_vars(ret, var_map)),
                effects: effects.clone(),
            },
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|e| self.substitute_named_vars(e, var_map)).collect())
            }
            Type::Array { elem, length } => Type::Array {
                elem: Box::new(self.substitute_named_vars(elem, var_map)),
                length: length.clone(),
            },
            Type::ForAll { vars, body } => Type::ForAll {
                vars: vars.clone(),
                body: Box::new(self.substitute_named_vars(body, var_map)),
            },
            Type::Resource { base, dimension } => Type::Resource {
                base: Box::new(self.substitute_named_vars(base, var_map)),
                dimension: *dimension,
            },
            Type::Refined { base, predicates } => Type::Refined {
                base: Box::new(self.substitute_named_vars(base, var_map)),
                predicates: predicates.clone(),
            },
            Type::Pi { param_name, param_type, body } => Type::Pi {
                param_name: param_name.clone(),
                param_type: Box::new(self.substitute_named_vars(param_type, var_map)),
                body: Box::new(self.substitute_named_vars(body, var_map)),
            },
            Type::Sigma { fst_name, fst_type, snd_type } => Type::Sigma {
                fst_name: fst_name.clone(),
                fst_type: Box::new(self.substitute_named_vars(fst_type, var_map)),
                snd_type: Box::new(self.substitute_named_vars(snd_type, var_map)),
            },
            Type::Session(s) => Type::Session(self.substitute_session_vars(s, var_map)),
        }
    }

    /// Substitute named type variables in a session type.
    fn substitute_session_vars(
        &self,
        s: &crate::types::SessionType,
        var_map: &HashMap<String, Type>,
    ) -> crate::types::SessionType {
        match s {
            crate::types::SessionType::Send(ty, cont) => crate::types::SessionType::Send(
                Box::new(self.substitute_named_vars(ty, var_map)),
                Box::new(self.substitute_session_vars(cont, var_map)),
            ),
            crate::types::SessionType::Recv(ty, cont) => crate::types::SessionType::Recv(
                Box::new(self.substitute_named_vars(ty, var_map)),
                Box::new(self.substitute_session_vars(cont, var_map)),
            ),
            crate::types::SessionType::Offer(branches) => crate::types::SessionType::Offer(
                branches
                    .iter()
                    .map(|(l, s)| (l.clone(), self.substitute_session_vars(s, var_map)))
                    .collect(),
            ),
            crate::types::SessionType::Select(branches) => crate::types::SessionType::Select(
                branches
                    .iter()
                    .map(|(l, s)| (l.clone(), self.substitute_session_vars(s, var_map)))
                    .collect(),
            ),
            crate::types::SessionType::End => crate::types::SessionType::End,
            crate::types::SessionType::Rec(v, body) => crate::types::SessionType::Rec(
                v.clone(),
                Box::new(self.substitute_session_vars(body, var_map)),
            ),
            crate::types::SessionType::RecVar(v) => crate::types::SessionType::RecVar(v.clone()),
        }
    }
}

impl Default for InferCtx {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InferCtx {
    fn clone(&self) -> Self {
        Self {
            bindings: self.bindings.clone(),
            parent: self.parent.clone(),
            next_var: self.next_var,
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Find the closest matching name for error suggestions (Levenshtein distance).
fn find_closest_match(target: &str, candidates: &[String]) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }

    let mut best = &candidates[0];
    let mut best_dist = levenshtein(target, &candidates[0]);

    for candidate in &candidates[1..] {
        let dist = levenshtein(target, candidate);
        if dist < best_dist {
            best_dist = dist;
            best = candidate;
        }
    }

    let max_dist = (target.len() / 3).max(3);
    if best_dist <= max_dist {
        Some(format!("did you mean '{}'?", best))
    } else {
        None
    }
}

/// Simple Levenshtein distance implementation.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (al, bl) = (a.len(), b.len());

    if al == 0 { return bl; }
    if bl == 0 { return al; }

    let mut prev: Vec<usize> = (0..=bl).collect();
    let mut curr = vec![0; bl + 1];

    for (i, ac) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, bc) in b.iter().enumerate() {
            let cost = if ac == bc { 0 } else { 1 };
            curr[j + 1] = (curr[j] + 1)
                .min(prev[j + 1] + 1)
                .min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[bl]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrimitiveType;

    #[test]
    fn test_fresh_variables_are_unique() {
        let mut ctx = InferCtx::new();
        let v1 = ctx.fresh_var();
        let v2 = ctx.fresh_var();
        assert_ne!(v1, v2);
    }

    #[test]
    fn test_instantiate_monomorphic() {
        let mut ctx = InferCtx::new();
        let scheme = TypeScheme::mono(UnifiedType::simple(
            Type::Primitive(PrimitiveType::Int),
        ));
        let ty = ctx.instantiate(&scheme);
        assert_eq!(ty, Type::Primitive(PrimitiveType::Int));
    }

    #[test]
    fn test_lookup_in_child_scope() {
        let mut parent = InferCtx::new();
        parent.insert(
            "x".to_string(),
            UnifiedType::simple(Type::Primitive(PrimitiveType::Int)),
        );
        let child = parent.child();
        assert!(child.lookup("x").is_some());
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
    }
}
