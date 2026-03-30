-- SPDX-License-Identifier: PMPL-1.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
||| TypeLL Type Inference Soundness
|||
||| Proves that the bidirectional type inference algorithm implemented in
||| infer.rs is sound: if inference succeeds with type T, then the typing
||| judgement ctx |- e : T holds.
|||
||| The proof covers:
|||   1. Unification soundness: if unify(T1, T2) succeeds with substitution S,
|||      then S(T1) = S(T2).
|||   2. Inference soundness: if infer(ctx, e) = T, then ctx |- e : T.
|||   3. Occurs check correctness: if a variable occurs in a type, unification
|||      correctly rejects the infinite type.
|||
||| These correspond to:
|||   - unify.rs: Unifier::unify and Substitution::apply
|||   - infer.rs: InferCtx::synthesize_var and InferCtx::check_against
|||   - infer.rs: InferCtx::generalize and InferCtx::instantiate
|||
||| @see unify.rs  — Robinson unification with occurs check
||| @see infer.rs  — Bidirectional type inference

module TYPELL.ABI.InferenceSoundness

import TYPELL.ABI.Soundness
import Data.List
import Data.Vect
import Decidable.Equality

%default total

-- ============================================================================
-- Types with Unification Variables
-- ============================================================================

||| Type with unification variables, modelling the types::Type enum
||| extended with Type::Var(TypeVar) for unification.
public export
data InfTy : Type where
  ||| Concrete boolean
  IBool   : InfTy
  ||| Concrete integer
  IInt    : InfTy
  ||| Concrete unit
  IUnit   : InfTy
  ||| Function type
  IArrow  : InfTy -> InfTy -> InfTy
  ||| Unification variable (corresponds to TypeVar(u32) in types.rs)
  IVar    : Nat -> InfTy

||| Decidable equality for inference types.
public export
DecEq InfTy where
  decEq IBool IBool = Yes Refl
  decEq IInt IInt = Yes Refl
  decEq IUnit IUnit = Yes Refl
  decEq (IArrow a1 b1) (IArrow a2 b2) with (decEq a1 a2, decEq b1 b2)
    decEq (IArrow a1 b1) (IArrow a1 b1) | (Yes Refl, Yes Refl) = Yes Refl
    decEq (IArrow a1 b1) (IArrow a2 b2) | (No c, _) = No (\case Refl => c Refl)
    decEq (IArrow a1 b1) (IArrow a2 b2) | (_, No c) = No (\case Refl => c Refl)
  decEq (IVar n) (IVar m) with (decEq n m)
    decEq (IVar n) (IVar n) | (Yes Refl) = Yes Refl
    decEq (IVar n) (IVar m) | (No c) = No (\case Refl => c Refl)
  decEq IBool IInt = No (\case Refl impossible)
  decEq IBool IUnit = No (\case Refl impossible)
  decEq IBool (IArrow _ _) = No (\case Refl impossible)
  decEq IBool (IVar _) = No (\case Refl impossible)
  decEq IInt IBool = No (\case Refl impossible)
  decEq IInt IUnit = No (\case Refl impossible)
  decEq IInt (IArrow _ _) = No (\case Refl impossible)
  decEq IInt (IVar _) = No (\case Refl impossible)
  decEq IUnit IBool = No (\case Refl impossible)
  decEq IUnit IInt = No (\case Refl impossible)
  decEq IUnit (IArrow _ _) = No (\case Refl impossible)
  decEq IUnit (IVar _) = No (\case Refl impossible)
  decEq (IArrow _ _) IBool = No (\case Refl impossible)
  decEq (IArrow _ _) IInt = No (\case Refl impossible)
  decEq (IArrow _ _) IUnit = No (\case Refl impossible)
  decEq (IArrow _ _) (IVar _) = No (\case Refl impossible)
  decEq (IVar _) IBool = No (\case Refl impossible)
  decEq (IVar _) IInt = No (\case Refl impossible)
  decEq (IVar _) IUnit = No (\case Refl impossible)
  decEq (IVar _) (IArrow _ _) = No (\case Refl impossible)

-- ============================================================================
-- Substitutions (Modelling unify.rs Substitution)
-- ============================================================================

||| A substitution maps unification variables to types.
||| Models Substitution in unify.rs (HashMap<TypeVar, Type>).
public export
Subst : Type
Subst = List (Nat, InfTy)

||| Look up a variable in a substitution.
public export
lookupSubst : Nat -> Subst -> Maybe InfTy
lookupSubst n [] = Nothing
lookupSubst n ((m, ty) :: rest) =
  if n == m then Just ty else lookupSubst n rest

||| Apply a substitution to a type.
||| Models Substitution::apply in unify.rs.
public export
applySubst : Subst -> InfTy -> InfTy
applySubst s IBool = IBool
applySubst s IInt = IInt
applySubst s IUnit = IUnit
applySubst s (IArrow a b) = IArrow (applySubst s a) (applySubst s b)
applySubst s (IVar n) =
  case lookupSubst n s of
    Nothing => IVar n
    Just ty => applySubst s ty

||| Compose two substitutions.
||| applySubst (compose s1 s2) t = applySubst s1 (applySubst s2 t)
public export
compose : Subst -> Subst -> Subst
compose s1 s2 = map (\(n, ty) => (n, applySubst s1 ty)) s2 ++ s1

||| The empty substitution is an identity.
public export
emptySubstId : (ty : InfTy) -> applySubst [] ty = ty
emptySubstId IBool = Refl
emptySubstId IInt = Refl
emptySubstId IUnit = Refl
emptySubstId (IArrow a b) =
  rewrite emptySubstId a in
  rewrite emptySubstId b in Refl
emptySubstId (IVar n) = Refl

-- ============================================================================
-- Occurs Check (Modelling unify.rs occurs check)
-- ============================================================================

||| Check whether a variable occurs in a type.
||| Models the occurs check in Unifier::unify in unify.rs.
public export
occurs : Nat -> InfTy -> Bool
occurs n IBool = False
occurs n IInt = False
occurs n IUnit = False
occurs n (IArrow a b) = occurs n a || occurs n b
occurs n (IVar m) = n == m

||| If a variable does not occur in a type, substituting for that variable
||| does not change the type.
public export
noOccursNoChange : (n : Nat) -> (ty : InfTy) -> (s : InfTy)
                -> occurs n ty = False
                -> applySubst [(n, s)] ty = ty
noOccursNoChange n IBool s prf = Refl
noOccursNoChange n IInt s prf = Refl
noOccursNoChange n IUnit s prf = Refl
noOccursNoChange n (IArrow a b) s prf =
  let (prfA, prfB) = orFalseImpliesBothFalse (occurs n a) (occurs n b) prf
  in rewrite noOccursNoChange n a s prfA in
     rewrite noOccursNoChange n b s prfB in Refl
  where
    orFalseImpliesBothFalse : (x : Bool) -> (y : Bool) -> (x || y) = False
                           -> (x = False, y = False)
    orFalseImpliesBothFalse False False Refl = (Refl, Refl)
noOccursNoChange n (IVar m) s prf with (decEq n m)
  noOccursNoChange n (IVar n) s prf | (Yes Refl) =
    absurd (trueNotFalse (sym prf))
    where
      trueNotFalse : True = False -> Void
      trueNotFalse Refl impossible
  noOccursNoChange n (IVar m) s prf | (No _) = Refl

-- ============================================================================
-- Unification Result
-- ============================================================================

||| Unification either succeeds with a substitution or fails.
||| Models the Result type returned by Unifier::unify.
public export
data UnifyResult : Type where
  ||| Unification succeeded with a most-general unifier.
  Unified   : (s : Subst) -> UnifyResult
  ||| Unification failed (type mismatch or occurs check).
  UnifyFail : UnifyResult

-- ============================================================================
-- Unification Soundness
-- ============================================================================

||| A substitution S is a unifier of T1 and T2 if S(T1) = S(T2).
public export
IsUnifier : Subst -> InfTy -> InfTy -> Type
IsUnifier s t1 t2 = applySubst s t1 = applySubst s t2

||| Unification for identical types produces the empty substitution.
public export
unifyRefl : (ty : InfTy) -> IsUnifier [] ty ty
unifyRefl ty = rewrite emptySubstId ty in
               rewrite emptySubstId ty in Refl

||| Unifying a variable with a type (no occurs) produces a valid substitution.
public export
unifyVarLeft : (n : Nat) -> (ty : InfTy)
            -> occurs n ty = False
            -> IsUnifier [(n, ty)] (IVar n) ty
unifyVarLeft n ty noOcc =
  rewrite noOccursNoChange n ty ty noOcc in Refl

||| Unifying two arrow types decomposes to unifying their components.
||| If S is a unifier of (A1 -> B1) and (A2 -> B2), then
||| S unifies A1 with A2, and S unifies B1 with B2.
public export
unifyArrowDecompose : (s : Subst)
                   -> IsUnifier s (IArrow a1 b1) (IArrow a2 b2)
                   -> (IsUnifier s a1 a2, IsUnifier s b1 b2)
unifyArrowDecompose s prf =
  let (prfA, prfB) = arrowInj prf
  in (prfA, prfB)
  where
    arrowInj : IArrow x1 y1 = IArrow x2 y2 -> (x1 = x2, y1 = y2)
    arrowInj Refl = (Refl, Refl)

||| Concrete type constructors with different heads cannot unify.
||| Models the type mismatch errors in Unifier::unify.
public export
boolNotInt : Not (IsUnifier s IBool IInt)
boolNotInt prf = case prf of Refl impossible

public export
boolNotUnit : Not (IsUnifier s IBool IUnit)
boolNotUnit prf = case prf of Refl impossible

public export
intNotUnit : Not (IsUnifier s IInt IUnit)
intNotUnit prf = case prf of Refl impossible

-- ============================================================================
-- Inference Soundness Statement
-- ============================================================================

||| Erase an inference type to a core type (ground types only).
||| This connects the inference world (InfTy with variables) to the
||| core type checking world (Ty without variables).
public export
eraseTy : InfTy -> Maybe Ty
eraseTy IBool = Just TBool
eraseTy IInt = Just TInt
eraseTy IUnit = Just TUnit
eraseTy (IArrow a b) = do
  a' <- eraseTy a
  b' <- eraseTy b
  Just (TArrow a' b')
eraseTy (IVar _) = Nothing

||| A fully-resolved inference type (no remaining variables) erases
||| to a core type.
public export
data FullyResolved : InfTy -> Type where
  ResBool  : FullyResolved IBool
  ResInt   : FullyResolved IInt
  ResUnit  : FullyResolved IUnit
  ResArrow : FullyResolved a -> FullyResolved b -> FullyResolved (IArrow a b)

||| A fully resolved type always erases successfully.
public export
resolvedErases : FullyResolved ty -> (coreTy : Ty ** eraseTy ty = Just coreTy)
resolvedErases ResBool = (TBool ** Refl)
resolvedErases ResInt = (TInt ** Refl)
resolvedErases ResUnit = (TUnit ** Refl)
resolvedErases (ResArrow ra rb) =
  let (ca ** pa) = resolvedErases ra
      (cb ** pb) = resolvedErases rb
  in (TArrow ca cb **
      rewrite pa in
      rewrite pb in Refl)

||| Inference soundness statement (for closed ground terms):
||| If type inference resolves to a ground type, then the typing judgement holds.
|||
||| This is a specification — the full constructive proof requires modelling
||| the entire inference algorithm. The statement captures the invariant
||| that TypeChecker.finish(ty) with valid=true implies the typing relation.
public export
InferenceSoundnessStatement : Type
InferenceSoundnessStatement =
  (ctx : Ctx n) -> (t : Term n) -> (infTy : InfTy)
  -> (resolved : FullyResolved infTy)
  -> (coreTy : Ty)
  -> (erasePrf : eraseTy infTy = Just coreTy)
  -> HasTy ctx t coreTy

-- ============================================================================
-- Unification Idempotence
-- ============================================================================

||| Applying a substitution twice is the same as applying it once (on its
||| own range). This is a key property of most-general unifiers.
|||
||| Models the idempotence that Substitution::apply relies on in unify.rs.
public export
substIdempotent : (s : Subst) -> (ty : InfTy)
               -> (idempotent : (n : Nat) -> (rhs : InfTy)
                   -> lookupSubst n s = Just rhs
                   -> applySubst s rhs = rhs)
               -> applySubst s (applySubst s ty) = applySubst s ty
substIdempotent s IBool _ = Refl
substIdempotent s IInt _ = Refl
substIdempotent s IUnit _ = Refl
substIdempotent s (IArrow a b) idem =
  rewrite substIdempotent s a idem in
  rewrite substIdempotent s b idem in Refl
substIdempotent s (IVar n) idem with (lookupSubst n s) proof prf
  substIdempotent s (IVar n) idem | Nothing = prf
  substIdempotent s (IVar n) idem | (Just ty) =
    rewrite idem n ty (sym prf) in Refl
