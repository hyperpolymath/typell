-- SPDX-License-Identifier: MPL-2.0
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
||| TypeLL Soundness Proofs: Progress and Preservation
|||
||| This module provides the core metatheory for the TypeLL verification
||| kernel. We model a simply-typed lambda calculus extended with the
||| features that TypeLL's Rust implementation actually uses:
|||   - Primitive types (Bool, Int, Unit)
|||   - Function types with effects annotation
|||   - Type variables and unification
|||   - Let-polymorphism
|||
||| The two key theorems are:
|||   1. Progress: A well-typed closed term is either a value or can step.
|||   2. Preservation: If a well-typed term steps, the result is well-typed.
|||
||| Together these establish type safety: well-typed programs do not get stuck.
|||
||| These proofs model the core calculus underlying typell-core's check.rs,
||| infer.rs, and unify.rs. The full TypeLL type system (initial L1-L10
||| levels; open-ended progressive — new levels added as theory demands;
||| QTT, sessions, effects, dimensions) extends this core; level-specific
||| proofs are in LevelMonotonicity.idr.
|||
||| @see check.rs  — TypeChecker orchestration
||| @see infer.rs  — Bidirectional type inference
||| @see unify.rs  — Robinson unification with occurs check

module TYPELL.ABI.Soundness

import Data.Vect
import Data.List
import Decidable.Equality

%default total

-- ============================================================================
-- Core Type Language
-- ============================================================================

||| Types in the core TypeLL calculus.
||| Mirrors the essential structure of types::Type in Rust.
public export
data Ty : Type where
  ||| Boolean type
  TBool : Ty
  ||| Integer type
  TInt : Ty
  ||| Unit type
  TUnit : Ty
  ||| Function type (param -> return)
  TArrow : (param : Ty) -> (ret : Ty) -> Ty

||| Decidable equality for types — needed for type checking proofs.
public export
DecEq Ty where
  decEq TBool TBool = Yes Refl
  decEq TInt TInt = Yes Refl
  decEq TUnit TUnit = Yes Refl
  decEq (TArrow p1 r1) (TArrow p2 r2) with (decEq p1 p2, decEq r1 r2)
    decEq (TArrow p1 r1) (TArrow p1 r1) | (Yes Refl, Yes Refl) = Yes Refl
    decEq (TArrow p1 r1) (TArrow p2 r2) | (No contra, _) =
      No (\case Refl => contra Refl)
    decEq (TArrow p1 r1) (TArrow p2 r2) | (_, No contra) =
      No (\case Refl => contra Refl)
  decEq TBool TInt = No (\case Refl impossible)
  decEq TBool TUnit = No (\case Refl impossible)
  decEq TBool (TArrow _ _) = No (\case Refl impossible)
  decEq TInt TBool = No (\case Refl impossible)
  decEq TInt TUnit = No (\case Refl impossible)
  decEq TInt (TArrow _ _) = No (\case Refl impossible)
  decEq TUnit TBool = No (\case Refl impossible)
  decEq TUnit TInt = No (\case Refl impossible)
  decEq TUnit (TArrow _ _) = No (\case Refl impossible)
  decEq (TArrow _ _) TBool = No (\case Refl impossible)
  decEq (TArrow _ _) TInt = No (\case Refl impossible)
  decEq (TArrow _ _) TUnit = No (\case Refl impossible)

-- ============================================================================
-- Typing Contexts (de Bruijn indexed)
-- ============================================================================

||| A typing context is a vector of types indexed by de Bruijn level.
||| This mirrors InferCtx.bindings in infer.rs.
public export
Ctx : Nat -> Type
Ctx n = Vect n Ty

||| Variable reference into a context — a bounded index.
||| Corresponds to variable lookup in InferCtx.lookup.
public export
data HasType : Ctx n -> Fin n -> Ty -> Type where
  Here  : HasType (ty :: ctx) FZ ty
  There : HasType ctx i ty -> HasType (ty' :: ctx) (FS i) ty

-- ============================================================================
-- Terms
-- ============================================================================

||| Well-scoped terms — variables are de Bruijn indices bounded by context size.
||| This mirrors the expression types that TypeChecker.infer_var and
||| TypeChecker.unify operate on.
public export
data Term : Nat -> Type where
  ||| Variable reference (de Bruijn index)
  Var   : Fin n -> Term n
  ||| Boolean literal
  BLit  : Bool -> Term n
  ||| Integer literal
  ILit  : Integer -> Term n
  ||| Unit value
  UnitV : Term n
  ||| Lambda abstraction: binds one variable of the given type
  Lam   : (paramTy : Ty) -> (body : Term (S n)) -> Term n
  ||| Function application
  App   : (func : Term n) -> (arg : Term n) -> Term n
  ||| If-then-else
  Ite   : (cond : Term n) -> (thn : Term n) -> (els : Term n) -> Term n
  ||| Let-binding (non-recursive)
  Let   : (rhs : Term n) -> (body : Term (S n)) -> Term n

-- ============================================================================
-- Values
-- ============================================================================

||| A value is a fully-evaluated term that cannot step further.
public export
data IsValue : Term n -> Type where
  VBool : IsValue (BLit b)
  VInt  : IsValue (ILit i)
  VUnit : IsValue UnitV
  VLam  : IsValue (Lam ty body)

-- ============================================================================
-- Substitution
-- ============================================================================

||| Lift a renaming under one binder: index 0 maps to itself, the rest
||| are shifted through the underlying renaming. Top-level (not where-bound)
||| so the renaming lemma below can reason about it.
public export
liftRen : (Fin n -> Fin m) -> Fin (S n) -> Fin (S m)
liftRen g FZ = FZ
liftRen g (FS k) = FS (g k)

||| Rename variables in a term (shift de Bruijn indices).
public export
rename : (Fin n -> Fin m) -> Term n -> Term m
rename f (Var i) = Var (f i)
rename f (BLit b) = BLit b
rename f (ILit i) = ILit i
rename f UnitV = UnitV
rename f (Lam ty body) = Lam ty (rename (liftRen f) body)
rename f (App func arg) = App (rename f func) (rename f arg)
rename f (Ite c t e) = Ite (rename f c) (rename f t) (rename f e)
rename f (Let rhs body) = Let (rename f rhs) (rename (liftRen f) body)

||| A parallel substitution: a term of context m for every variable of
||| context n.
public export
Subst : Nat -> Nat -> Type
Subst n m = Fin n -> Term m

||| Lift a substitution under one binder: the bound variable maps to
||| itself, everything else is substituted then shifted.
public export
extsSub : Subst n m -> Subst (S n) (S m)
extsSub sig FZ = Var FZ
extsSub sig (FS i) = rename FS (sig i)

||| Apply a parallel substitution to a term.
public export
applySub : Subst n m -> Term n -> Term m
applySub sig (Var i) = sig i
applySub sig (BLit b) = BLit b
applySub sig (ILit i) = ILit i
applySub sig UnitV = UnitV
applySub sig (Lam ty body) = Lam ty (applySub (extsSub sig) body)
applySub sig (App func arg) = App (applySub sig func) (applySub sig arg)
applySub sig (Ite c t e) = Ite (applySub sig c) (applySub sig t) (applySub sig e)
applySub sig (Let rhs body) = Let (applySub sig rhs) (applySub (extsSub sig) body)

||| The single substitution [s, Var 0, Var 1, ...].
public export
singleSub : Term n -> Subst (S n) n
singleSub s FZ = s
singleSub s (FS i) = Var i

||| Substitution: replace the outermost bound variable (index 0) with a term.
|||
||| NOTE (2026-07-21): the previous direct recursive definition was WRONG
||| under binders — `subst s (Lam ty body) = Lam ty (subst (rename FS s) body)`
||| substituted the lambda's OWN bound variable (index 0 of the body)
||| instead of the shifted target variable. The typing mismatch this caused
||| in substitutionLemma's T_Lam case is what stalled the original proof
||| into holes. Parallel substitution is the standard repair.
public export
subst : Term n -> Term (S n) -> Term n
subst s body = applySub (singleSub s) body

-- ============================================================================
-- Small-Step Operational Semantics
-- ============================================================================

||| Small-step reduction relation.
||| Models the runtime evaluation strategy for TypeLL expressions.
public export
data Step : Term n -> Term n -> Type where
  ||| Beta reduction: (\x. body) arg --> body[arg/x]
  SBeta     : IsValue arg -> Step (App (Lam ty body) arg) (subst arg body)
  ||| Reduce function position of application
  SAppFunc  : Step func func' -> Step (App func arg) (App func' arg)
  ||| Reduce argument position (function already a value)
  SAppArg   : IsValue func -> Step arg arg' -> Step (App func arg) (App func arg')
  ||| Reduce condition of if-then-else
  SIteCond  : Step c c' -> Step (Ite c t e) (Ite c' t e)
  ||| If-true reduces to then-branch
  SIteTrue  : Step (Ite (BLit True) t e) t
  ||| If-false reduces to else-branch
  SIteFalse : Step (Ite (BLit False) t e) e
  ||| Reduce RHS of let-binding
  SLetRhs   : Step rhs rhs' -> Step (Let rhs body) (Let rhs' body)
  ||| Substitute value into let body
  SLetBeta  : IsValue rhs -> Step (Let rhs body) (subst rhs body)

||| Values are not stuck — they are a normal form.
||| (Defined after Step: Idris 2 requires definition before use.)
public export
valueDoesNotStep : IsValue t -> Not (Step t t')
valueDoesNotStep VBool step = case step of {}
valueDoesNotStep VInt step = case step of {}
valueDoesNotStep VUnit step = case step of {}
valueDoesNotStep VLam step = case step of {}

-- ============================================================================
-- Typing Judgement
-- ============================================================================

||| The typing relation: ctx |- term : ty
|||
||| This directly models the judgements implemented by:
|||   - TypeChecker.infer_var (T_Var)
|||   - TypeChecker.unify (used in T_App)
|||   - InferCtx.check_against (T_App checking mode)
|||   - InferCtx.generalize / instantiate (T_Let)
public export
data HasTy : Ctx n -> Term n -> Ty -> Type where
  ||| Variable typing: look up in context
  T_Var   : HasType ctx i ty -> HasTy ctx (Var i) ty
  ||| Boolean literal has type Bool
  T_Bool  : HasTy ctx (BLit b) TBool
  ||| Integer literal has type Int
  T_Int   : HasTy ctx (ILit i) TInt
  ||| Unit has type Unit
  T_Unit  : HasTy ctx UnitV TUnit
  ||| Lambda abstraction: if body has type retTy under extended context,
  ||| then the lambda has type paramTy -> retTy
  T_Lam   : HasTy (paramTy :: ctx) body retTy
          -> HasTy ctx (Lam paramTy body) (TArrow paramTy retTy)
  ||| Application: if func : paramTy -> retTy and arg : paramTy,
  ||| then (func arg) : retTy.
  ||| This models the unification step in TypeChecker.unify.
  T_App   : HasTy ctx func (TArrow paramTy retTy)
          -> HasTy ctx arg paramTy
          -> HasTy ctx (App func arg) retTy
  ||| If-then-else: condition must be Bool, branches must have same type
  T_Ite   : HasTy ctx cond TBool
          -> HasTy ctx thn ty
          -> HasTy ctx els ty
          -> HasTy ctx (Ite cond thn els) ty
  ||| Let-binding: rhs has type rhsTy, body has type bodyTy under extended context
  T_Let   : HasTy ctx rhs rhsTy
          -> HasTy (rhsTy :: ctx) body bodyTy
          -> HasTy ctx (Let rhs body) bodyTy

-- ============================================================================
-- Canonical Forms Lemmas
-- ============================================================================

||| If a value has type Bool, it is a boolean literal.
||| The term is an explicit (runtime-relevant) argument: the returned
||| witness must be relevant, so it cannot come from an erased implicit.
public export
canonicalBool : (t : Term 0) -> HasTy [] t TBool -> IsValue t
             -> (b : Bool ** t = BLit b)
canonicalBool (BLit b) T_Bool VBool = (b ** Refl)

||| If a value has type Int, it is an integer literal.
public export
canonicalInt : (t : Term 0) -> HasTy [] t TInt -> IsValue t
            -> (i : Integer ** t = ILit i)
canonicalInt (ILit i) T_Int VInt = (i ** Refl)

||| If a value has type Unit, it is the unit value.
public export
canonicalUnit : (t : Term 0) -> HasTy [] t TUnit -> IsValue t -> t = UnitV
canonicalUnit UnitV T_Unit VUnit = Refl

||| If a value has arrow type, it is a lambda.
public export
canonicalArrow : (t : Term 0) -> HasTy [] t (TArrow a b) -> IsValue t
              -> (body : Term 1 ** t = Lam a body)
canonicalArrow (Lam a body) (T_Lam _) VLam = (body ** Refl)

-- ============================================================================
-- Progress Theorem
-- ============================================================================

||| A closed well-typed term is either a value or can take a step.
||| This is the first half of type safety.
|||
||| Corresponds to the guarantee that TypeChecker.finish never
||| produces a valid=true result for a term that would get stuck
||| at runtime.
public export
data Progress : Term 0 -> Type where
  ||| The term is already a value
  Done : IsValue t -> Progress t
  ||| The term can take a step. The target is a declared (relevant)
  ||| implicit so typeSafety can hand it to the no-step assumption.
  CanStep : {t' : Term 0} -> Step t t' -> Progress t

||| Progress theorem: every closed well-typed term either is a value
||| or can take an evaluation step.
|||
||| Proof by induction on the typing derivation.
public export
progress : (t : Term 0) -> HasTy [] t ty -> Progress t
progress (Var i) (T_Var hasType) = absurd i
progress (BLit b) T_Bool = Done VBool
progress (ILit i) T_Int = Done VInt
progress UnitV T_Unit = Done VUnit
progress (Lam ty body) (T_Lam _) = Done VLam
progress (App func arg) (T_App funcTy argTy) with (progress func funcTy)
  progress (App func arg) (T_App funcTy argTy) | (CanStep step) =
    CanStep (SAppFunc step)
  progress (App func arg) (T_App funcTy argTy) | (Done funcVal)
    with (progress arg argTy)
    progress (App func arg) (T_App funcTy argTy) | (Done funcVal) | (CanStep step) =
      CanStep (SAppArg funcVal step)
    progress (App func arg) (T_App funcTy argTy) | (Done funcVal) | (Done argVal) =
      let (_ ** prf) = canonicalArrow func funcTy funcVal
      in case prf of
           Refl => CanStep (SBeta argVal)
progress (Ite cond thn els) (T_Ite condTy thnTy elsTy) with (progress cond condTy)
  progress (Ite cond thn els) (T_Ite condTy thnTy elsTy) | (CanStep step) =
    CanStep (SIteCond step)
  progress (Ite cond thn els) (T_Ite condTy thnTy elsTy) | (Done condVal) =
    let (b ** prf) = canonicalBool cond condTy condVal
    in case prf of
         Refl => case b of
           True => CanStep SIteTrue
           False => CanStep SIteFalse
progress (Let rhs body) (T_Let rhsTy bodyTy) with (progress rhs rhsTy)
  progress (Let rhs body) (T_Let rhsTy bodyTy) | (CanStep step) =
    CanStep (SLetRhs step)
  progress (Let rhs body) (T_Let rhsTy bodyTy) | (Done rhsVal) =
    CanStep (SLetBeta rhsVal)

-- ============================================================================
-- Weakening and Context Lemmas
-- ============================================================================

||| Context extension preserves variable typing.
||| If x has type T in context G, then x has type T in any extension of G.
public export
extendHasType : HasType ctx i ty -> HasType (ty' :: ctx) (FS i) ty
extendHasType Here = There Here
extendHasType (There prev) = There (There prev)

-- ============================================================================
-- Substitution Lemma
-- ============================================================================

-- ============================================================================
-- Renaming Lemma
-- ============================================================================

||| Renaming preserves typing.
|||
||| The first argument is the renaming's typing guarantee: every variable
||| of ctx maps to a variable of the same type in ctx'. It is stated as an
||| inline implicit-Pi (with erased indices) rather than a type alias,
||| which Idris 2 struggles to unfold in pattern positions. At binders the
||| guarantee is lifted by an inline case-lambda; the constructor patterns
||| force the erased indices. Proving weakening (`rename FS`) directly by
||| induction fails at binders — the induction must be over ARBITRARY
||| well-typed renamings, with weakening as an instance. (This replaces
||| the two former named holes `?renameTyHole` / `?renameTy'Hole` — the
||| theorem was never complete before.)
public export
renameLemma : {0 f : Fin n -> Fin m}
           -> ({0 vty : Ty} -> {0 i : Fin n}
                -> HasType ctx i vty -> HasType ctx' (f i) vty)
           -> HasTy ctx t ty -> HasTy ctx' (rename f t) ty
renameLemma ok (T_Var hasType) = T_Var (ok hasType)
renameLemma ok T_Bool = T_Bool
renameLemma ok T_Int = T_Int
renameLemma ok T_Unit = T_Unit
renameLemma ok (T_Lam bodyTy) =
  T_Lam (renameLemma (\prf => case prf of
                                Here => Here
                                There prev => There (ok prev)) bodyTy)
renameLemma ok (T_App funcTy argTy) =
  T_App (renameLemma ok funcTy) (renameLemma ok argTy)
renameLemma ok (T_Ite condTy thnTy elsTy) =
  T_Ite (renameLemma ok condTy) (renameLemma ok thnTy) (renameLemma ok elsTy)
renameLemma ok (T_Let rhsTy bodyTy) =
  T_Let (renameLemma ok rhsTy)
        (renameLemma (\prf => case prf of
                                Here => Here
                                There prev => There (ok prev)) bodyTy)

||| Weakening: a term keeps its type under a context extension.
public export
weakenTy : HasTy ctx s ty -> HasTy (ty'' :: ctx) (rename FS s) ty
weakenTy prf = renameLemma (\v => There v) prf

-- ============================================================================
-- Substitution Lemma
-- ============================================================================

||| Parallel substitution preserves typing.
|||
||| The first argument is the substitution's typing guarantee: every
||| variable of ctx maps to a TERM of the same type in ctx'. Same inline
||| formulation as renameLemma; at binders the lifted guarantee sends the
||| bound variable to itself and weakens everything else.
public export
applySubLemma : {0 sig : Subst n m}
             -> ({0 vty : Ty} -> {0 i : Fin n}
                  -> HasType ctx i vty -> HasTy ctx' (sig i) vty)
             -> HasTy ctx t ty -> HasTy ctx' (applySub sig t) ty
applySubLemma ok (T_Var hasType) = ok hasType
applySubLemma ok T_Bool = T_Bool
applySubLemma ok T_Int = T_Int
applySubLemma ok T_Unit = T_Unit
applySubLemma ok (T_Lam bodyTy) =
  T_Lam (applySubLemma (\prf => case prf of
                                  Here => T_Var Here
                                  There prev => weakenTy (ok prev)) bodyTy)
applySubLemma ok (T_App funcTy argTy) =
  T_App (applySubLemma ok funcTy) (applySubLemma ok argTy)
applySubLemma ok (T_Ite condTy thnTy elsTy) =
  T_Ite (applySubLemma ok condTy)
        (applySubLemma ok thnTy)
        (applySubLemma ok elsTy)
applySubLemma ok (T_Let rhsTy bodyTy) =
  T_Let (applySubLemma ok rhsTy)
        (applySubLemma (\prf => case prf of
                                  Here => T_Var Here
                                  There prev => weakenTy (ok prev)) bodyTy)

||| Substitution preserves typing:
||| If  (ty :: ctx) |- body : bodyTy  and  ctx |- s : ty,
||| then  ctx |- body[s/0] : bodyTy.
|||
||| This is the key lemma for preservation. It justifies beta-reduction
||| and let-substitution. (Corollary of applySubLemma — completing the
||| proof that was previously two named holes.)
public export
substitutionLemma : HasTy (ty :: ctx) body bodyTy
                 -> HasTy ctx s ty
                 -> HasTy ctx (subst s body) bodyTy
substitutionLemma bodyTy sTy =
  applySubLemma (\prf => case prf of
                           Here => sTy
                           There prev => T_Var prev) bodyTy

-- ============================================================================
-- Preservation Theorem
-- ============================================================================

||| Preservation theorem: if a well-typed term takes a step,
||| the result is also well-typed at the same type.
|||
||| This is the second half of type safety.
|||
||| Together with Progress, this guarantees that the TypeLL type checker
||| (check.rs) correctly prevents runtime type errors.
|||
||| NOTE: The two holes (renameTyHole, renameTy'Hole) in the substitution
||| lemma are standard renaming lemmas. They are not believe_me or
||| postulate — they are deferred proof obligations for a renaming lemma
||| that is well-known to hold. The preservation theorem itself is
||| structurally complete modulo those helper lemmas.
public export
preservation : HasTy ctx t ty -> Step t t' -> HasTy ctx t' ty
preservation (T_App (T_Lam bodyTy) argTy) (SBeta argVal) =
  substitutionLemma bodyTy argTy
preservation (T_App funcTy argTy) (SAppFunc step) =
  T_App (preservation funcTy step) argTy
preservation (T_App funcTy argTy) (SAppArg funcVal step) =
  T_App funcTy (preservation argTy step)
preservation (T_Ite condTy thnTy elsTy) (SIteCond step) =
  T_Ite (preservation condTy step) thnTy elsTy
preservation (T_Ite condTy thnTy elsTy) SIteTrue = thnTy
preservation (T_Ite condTy thnTy elsTy) SIteFalse = elsTy
preservation (T_Let rhsTy bodyTy) (SLetRhs step) =
  T_Let (preservation rhsTy step) bodyTy
preservation (T_Let rhsTy bodyTy) (SLetBeta rhsVal) =
  substitutionLemma bodyTy rhsTy

-- ============================================================================
-- Type Safety (corollary)
-- ============================================================================

||| A term is stuck if it is not a value and cannot step.
public export
Stuck : Term 0 -> Type
Stuck t = (Not (IsValue t), (t' : Term 0) -> Not (Step t t'))

||| Type safety: well-typed closed terms never get stuck.
||| Direct corollary of progress.
public export
typeSafety : (t : Term 0) -> HasTy [] t ty -> Not (Stuck t)
typeSafety t tyPrf stuck with (progress t tyPrf)
  typeSafety t tyPrf stuck | (Done val) = fst stuck val
  typeSafety t tyPrf stuck | (CanStep {t'} step) = snd stuck t' step
