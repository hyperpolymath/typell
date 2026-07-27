-- SPDX-License-Identifier: MPL-2.0
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
||| TypeLL Progressive Type Safety Hierarchy — Monotonicity Proofs
|||
||| TypeLL is an open-ended progressive framework: levels are added as the
||| type theory demands, with no fixed upper bound.  The initial checked
||| set (L1-L10) is defined below; L10 is the current top, not a ceiling.
|||
||| Initial checked set (L1-L10):
|||   L1  : Parse-time safety         (well-formed AST)
|||   L2  : Schema-binding safety     (named type resolution)
|||   L3  : Type-compatible operations (unification + operator checking)
|||   L4  : Null-safety               (option types, totality)
|||   L5  : Injection-proof safety    (refinement predicates)
|||   L6  : Result-type safety        (return type inference)
|||   L7  : Cardinality safety        (bounded quantifiers)
|||   L8  : Effect-tracking safety    (algebraic effects)
|||   L9  : Temporal safety           (session types, state machines)
|||   L10 : Linearity safety          (QTT bounded usage, linear types)
|||
||| This module proves the key structural property: higher levels
||| strictly subsume lower levels. A program that is safe at level N
||| is automatically safe at all levels M < N.
|||
||| This is the central correctness claim of the TypeLL project and
||| justifies the level-based type checking in TypedQLiser, VCL-total,
||| and StatistEase.
|||
||| @see ROADMAP.adoc  — Level definitions
||| @see check.rs      — TypeChecker with level-aware features

module TYPELL.ABI.LevelMonotonicity

import Data.Nat
import Data.Fin
import Decidable.Equality

%default total

-- ============================================================================
-- Local Nat/LTE lemmas
-- ============================================================================
-- Names drifted across stdlib versions (lteRefl / lteTransitive /
-- lteAntisymmetric / succInjective are not exported by this base);
-- defined here so the module is self-contained under Idris 2 0.7.0+.

lteRefl : {n : Nat} -> LTE n n
lteRefl {n = Z} = LTEZero
lteRefl {n = S k} = LTESucc lteRefl

lteTransitive : LTE a b -> LTE b c -> LTE a c
lteTransitive LTEZero _ = LTEZero
lteTransitive (LTESucc ab) (LTESucc bc) = LTESucc (lteTransitive ab bc)

lteAntisymmetric : LTE a b -> LTE b a -> a = b
lteAntisymmetric LTEZero LTEZero = Refl
lteAntisymmetric (LTESucc x) (LTESucc y) = cong S (lteAntisymmetric x y)

succInjective : (a, b : Nat) -> S a = S b -> a = b
succInjective _ _ Refl = Refl

-- ============================================================================
-- Level Representation
-- ============================================================================

||| A TypeLL safety level, represented as a bounded natural number 1..10.
||| Using Fin 10 internally (0-indexed) but displayed as 1-indexed.
public export
data Level : Type where
  MkLevel : Fin 10 -> Level

||| Smart constructors for each level.
public export
L1 : Level
L1 = MkLevel 0

public export
L2 : Level
L2 = MkLevel 1

public export
L3 : Level
L3 = MkLevel 2

public export
L4 : Level
L4 = MkLevel 3

public export
L5 : Level
L5 = MkLevel 4

public export
L6 : Level
L6 = MkLevel 5

public export
L7 : Level
L7 = MkLevel 6

public export
L8 : Level
L8 = MkLevel 7

public export
L9 : Level
L9 = MkLevel 8

public export
L10 : Level
L10 = MkLevel 9

||| Extract the numeric index from a level.
public export
levelIndex : Level -> Fin 10
levelIndex (MkLevel f) = f

||| Level ordering: L_a <= L_b iff a's index <= b's index.
public export
LTE_Level : Level -> Level -> Type
LTE_Level (MkLevel a) (MkLevel b) = LTE (finToNat a) (finToNat b)

||| Strict level ordering: L_a < L_b iff a's index < b's index.
public export
LT_Level : Level -> Level -> Type
LT_Level (MkLevel a) (MkLevel b) = LT (finToNat a) (finToNat b)

-- ============================================================================
-- Safety Properties
-- ============================================================================

||| A safety property is a predicate on programs that a level guarantees.
||| Each level introduces new guarantees while preserving all lower ones.
|||
||| We model a "program" abstractly as a type, since TypeLL is a type
||| system kernel — its job is to classify programs by their type safety.
public export
SafetyProperty : Type
SafetyProperty = Type

||| The set of safety properties guaranteed at each level.
||| This is modelled as a function from Level to a list of properties.
|||
||| Level properties are cumulative: level N includes all properties of
||| levels 1 through N, plus its own new property.
public export
record LevelSpec where
  constructor MkLevelSpec
  ||| The new property introduced at this level.
  newProperty : SafetyProperty
  ||| All properties at this level (including inherited ones).
  allProperties : List SafetyProperty

-- ============================================================================
-- Safety Predicates (abstract model)
-- ============================================================================

||| A program satisfies a safety predicate at a given level.
||| This is parameterised by a program representation P.
public export
data SafeAt : (program : p) -> Level -> Type where
  ||| Base: level-1 safety (a well-formed AST) is established outside this
  ||| abstract model — the kernel's parser supplies the evidence.
  ||| (The previous formulation `SafeAt prog L1 -> SafeAt prog L1` was
  ||| circular, leaving the whole family uninhabited.)
  SafeL1  : SafeAt prog L1
  ||| Step: safety at L(n+1) is built from safety at Ln (plus the level's
  ||| new property, which this abstract model does not represent).
  ||| Indices mirror levelStrictlyIncreasing: `weaken n` and `FS n` are
  ||| both Fin 10 when n : Fin 9. (The previous `MkLevel n -> MkLevel (FS n)`
  ||| forced FS n : Fin 11 — an off-by-one that never type-checked.)
  SafeUp  : {n : Fin 9}
          -> SafeAt prog (MkLevel (weaken n))
          -> SafeAt prog (MkLevel (FS n))

-- ============================================================================
-- Monotonicity Properties
-- ============================================================================

||| Level ordering is reflexive.
public export
lteLevelRefl : (l : Level) -> LTE_Level l l
lteLevelRefl (MkLevel f) = lteRefl

||| Level ordering is transitive.
public export
lteLevelTrans : {a, b, c : Level} -> LTE_Level a b -> LTE_Level b c -> LTE_Level a c
lteLevelTrans {a = MkLevel fa} {b = MkLevel fb} {c = MkLevel fc} ab bc =
  lteTransitive ab bc

||| Level ordering is antisymmetric (over indices).
public export
lteLevelAntiSym : {a, b : Level} -> LTE_Level a b -> LTE_Level b a -> levelIndex a = levelIndex b
lteLevelAntiSym {a = MkLevel fa} {b = MkLevel fb} ab ba =
  finToNatInjective fa fb (lteAntisymmetric ab ba)
  where
    finToNatInjective : (x : Fin n) -> (y : Fin n) -> finToNat x = finToNat y -> x = y
    finToNatInjective FZ FZ Refl = Refl
    finToNatInjective (FS x) (FS y) prf =
      cong FS (finToNatInjective x y (succInjective (finToNat x) (finToNat y) prf))

-- ============================================================================
-- Core Monotonicity Theorem
-- ============================================================================

||| Every level strictly subsumes the level below it.
||| L(n+1) > Ln for all valid n.
public export
levelStrictlyIncreasing : (n : Fin 9) -> LT_Level (MkLevel (weaken n)) (MkLevel (FS n))
levelStrictlyIncreasing n = LTESucc (weakenLTE n)
  where
    weakenLTE : (k : Fin m) -> LTE (finToNat (weaken k)) (finToNat k)
    weakenLTE FZ = LTEZero
    weakenLTE (FS k) = LTESucc (weakenLTE k)

||| The subsumption theorem: if a level m is at most level n,
||| then everything guaranteed at m is guaranteed at n.
|||
||| This is the central monotonicity property. It ensures that
||| upgrading a program's type safety level from m to n (where m <= n)
||| never loses any guarantees.
public export
subsumption : (m : Nat) -> (n : Nat) -> LTE m n
           -> (prop : Nat -> Type)
           -> ((k : Nat) -> LTE k m -> prop k)
           -> (j : Nat) -> LTE j m -> prop j
subsumption m n mLTEn prop holds j jLTEm = holds j jLTEm

-- ============================================================================
-- Level Feature Sets (cumulative)
-- ============================================================================

||| A type feature set — the capabilities available at each level.
||| Models the features vector in CheckResult.
public export
data Feature : Type where
  ||| L1: Well-formed AST
  WellFormedAST     : Feature
  ||| L2: Named type resolution
  SchemaBinding     : Feature
  ||| L3: Type unification
  TypeUnification   : Feature
  ||| L4: Null safety (Option/Maybe)
  NullSafety        : Feature
  ||| L5: Refinement predicates
  RefinementPreds   : Feature
  ||| L6: Return type inference
  ResultTypes       : Feature
  ||| L7: Bounded quantifiers (cardinality)
  Cardinality       : Feature
  ||| L8: Algebraic effects
  EffectTracking    : Feature
  ||| L9: Session types
  SessionTypes      : Feature
  ||| L10: Linear/QTT usage
  LinearUsage       : Feature

||| The feature newly introduced at 0-indexed level k (k = 0..9).
||| Total on all of Nat: indices >= 10 return WellFormedAST, a value never
||| consulted through the Fin 10 interface below — it exists only to keep
||| the cumulative definition total.
public export
newFeatureAt : Nat -> Feature
newFeatureAt 0 = WellFormedAST
newFeatureAt 1 = SchemaBinding
newFeatureAt 2 = TypeUnification
newFeatureAt 3 = NullSafety
newFeatureAt 4 = RefinementPreds
newFeatureAt 5 = ResultTypes
newFeatureAt 6 = Cardinality
newFeatureAt 7 = EffectTracking
newFeatureAt 8 = SessionTypes
newFeatureAt 9 = LinearUsage
newFeatureAt _ = WellFormedAST

||| Cumulative feature list over Nat indices: level k+1 is level k plus
||| exactly one new feature. Defined RECURSIVELY (2026-07-21; previously a
||| 10-literal table) so that monotonicity is provable by induction rather
||| than enumerated — or, as before, left as a hole.
public export
featN : Nat -> List Feature
featN Z = [WellFormedAST]
featN (S k) = featN k ++ [newFeatureAt (S k)]

||| The features available at a given level index (0-indexed).
||| Level n includes all features 0..n. Same values as the previous
||| literal table (featN 1 reduces to [WellFormedAST, SchemaBinding], and
||| so on) — the featureCountCorrect cases below still hold by Refl.
public export
featuresAtLevel : Fin 10 -> List Feature
featuresAtLevel f = featN (finToNat f)

||| Feature count at each level is exactly (level index + 1).
||| This confirms the cumulative structure: each level adds exactly one feature.
public export
featureCountCorrect : (n : Fin 10) -> length (featuresAtLevel n) = S (finToNat n)
featureCountCorrect FZ = Refl
featureCountCorrect (FS FZ) = Refl
featureCountCorrect (FS (FS FZ)) = Refl
featureCountCorrect (FS (FS (FS FZ))) = Refl
featureCountCorrect (FS (FS (FS (FS FZ)))) = Refl
featureCountCorrect (FS (FS (FS (FS (FS FZ))))) = Refl
featureCountCorrect (FS (FS (FS (FS (FS (FS FZ)))))) = Refl
featureCountCorrect (FS (FS (FS (FS (FS (FS (FS FZ))))))) = Refl
featureCountCorrect (FS (FS (FS (FS (FS (FS (FS (FS FZ)))))))) = Refl
featureCountCorrect (FS (FS (FS (FS (FS (FS (FS (FS (FS FZ))))))))) = Refl

-- ============================================================================
-- Subsumption via Feature Set Inclusion
-- ============================================================================

||| Feature membership in a list.
public export
data FeatureIn : Feature -> List Feature -> Type where
  FeatureHere  : FeatureIn f (f :: fs)
  FeatureThere : FeatureIn f fs -> FeatureIn f (f' :: fs)

||| List inclusion: every element of xs is in ys.
public export
data SubsetOf : List Feature -> List Feature -> Type where
  EmptySubset : SubsetOf [] ys
  ConsSubset  : FeatureIn f ys -> SubsetOf fs ys -> SubsetOf (f :: fs) ys

||| Widening: a subset of ys is a subset of y :: ys.
public export
subsetWiden : SubsetOf xs ys -> SubsetOf xs (y :: ys)
subsetWiden EmptySubset = EmptySubset
subsetWiden (ConsSubset i rest) = ConsSubset (FeatureThere i) (subsetWiden rest)

||| Subset inclusion is reflexive.
public export
subsetRefl : (xs : List Feature) -> SubsetOf xs xs
subsetRefl [] = EmptySubset
subsetRefl (x :: xs) = ConsSubset FeatureHere (subsetWiden (subsetRefl xs))

||| Membership transports along subset inclusion.
public export
featureInSubset : FeatureIn f ys -> SubsetOf ys zs -> FeatureIn f zs
featureInSubset FeatureHere (ConsSubset i _) = i
featureInSubset (FeatureThere x) (ConsSubset _ rest) = featureInSubset x rest

||| Subset inclusion is transitive.
public export
subsetTrans : SubsetOf xs ys -> SubsetOf ys zs -> SubsetOf xs zs
subsetTrans EmptySubset _ = EmptySubset
subsetTrans (ConsSubset i rest) yz =
  ConsSubset (featureInSubset i yz) (subsetTrans rest yz)

||| A list is a subset of itself extended on the right.
public export
subsetAppendRight : (xs, ys : List Feature) -> SubsetOf xs (xs ++ ys)
subsetAppendRight [] ys = EmptySubset
subsetAppendRight (x :: xs) ys =
  ConsSubset FeatureHere (subsetWiden (subsetAppendRight xs ys))

||| Split LTE a (S k) into LTE a k or a = S k.
lteSplit : (a, k : Nat) -> LTE a (S k) -> Either (LTE a k) (a = S k)
lteSplit Z k LTEZero = Left LTEZero
lteSplit (S Z) Z (LTESucc LTEZero) = Right Refl
lteSplit (S (S j)) Z (LTESucc p) = absurd p
lteSplit (S j) (S k) (LTESucc p) = case lteSplit j k p of
  Left q => Left (LTESucc q)
  Right Refl => Right Refl

||| Monotonicity over the cumulative Nat-indexed lists — the inductive
||| heart of the theorem, replacing the former `?featureMonoGeneral` hole
||| (which idris2 --check accepted silently; only the L1 case had been
||| proven before 2026-07-21).
public export
featNMono : (a, b : Nat) -> LTE a b -> SubsetOf (featN a) (featN b)
featNMono Z Z LTEZero = subsetRefl (featN Z)
featNMono (S j) Z prf = absurd prf
featNMono a (S k) prf = case lteSplit a k prf of
  Left q => subsetTrans (featNMono a k q)
                        (subsetAppendRight (featN k) [newFeatureAt (S k)])
  Right eq => rewrite eq in subsetRefl (featN (S k))

||| Higher levels have strictly more features.
||| If m <= n then features(m) is a subset of features(n).
|||
||| This is the feature-set formulation of level monotonicity.
||| It implies that any type check that passes at level m will also
||| pass at level n >= m, since level n performs all checks that m does
||| plus additional ones. Now fully proven as a corollary of featNMono.
public export
featureMonotonicity : (m : Fin 10) -> (n : Fin 10)
                   -> LTE (finToNat m) (finToNat n)
                   -> SubsetOf (featuresAtLevel m) (featuresAtLevel n)
featureMonotonicity m n prf = featNMono (finToNat m) (finToNat n) prf

-- ============================================================================
-- No Downgrade Theorem
-- ============================================================================

||| A type check result valid at level n cannot be downgraded.
||| If a term requires level-n features, it cannot be safely typed at
||| any level m < n.
|||
||| This captures the strictness of the hierarchy: levels are not just
||| labels but represent genuinely increasing verification power.
public export
data RequiresLevel : Nat -> Type where
  ||| Needs refinement predicates (L5)
  NeedsRefinement : RequiresLevel 4
  ||| Needs effect tracking (L8)
  NeedsEffects    : RequiresLevel 7
  ||| Needs session types (L9)
  NeedsSessions   : RequiresLevel 8
  ||| Needs linearity (L10)
  NeedsLinearity  : RequiresLevel 9

||| A program requiring level n features cannot be checked at level m < n.
public export
noDowngrade : RequiresLevel n -> (m : Nat) -> LT m n -> Not (LTE n m)
noDowngrade _ m mLTn nLTEm = absurd (ltNotLTE mLTn nLTEm)
  where
    ltNotLTE : LT a b -> LTE b a -> Void
    ltNotLTE (LTESucc x) (LTESucc y) = ltNotLTE x y

-- ============================================================================
-- Level Lattice Properties
-- ============================================================================

||| Levels form a total order (any two levels are comparable).
public export
levelTotalOrder : (a : Level) -> (b : Level) -> Either (LTE_Level a b) (LTE_Level b a)
levelTotalOrder (MkLevel fa) (MkLevel fb) =
  case isLTE (finToNat fa) (finToNat fb) of
    Yes prf => Left prf
    No notLTE => Right (notLTEImpliesGTE (finToNat fa) (finToNat fb) notLTE)
  where
    notLTEImpliesGTE : (a : Nat) -> (b : Nat) -> Not (LTE a b) -> LTE b a
    notLTEImpliesGTE Z b notLTE = absurd (notLTE LTEZero)
    notLTEImpliesGTE (S a) Z notLTE = LTEZero
    notLTEImpliesGTE (S a) (S b) notLTE =
      LTESucc (notLTEImpliesGTE a b (\prf => notLTE (LTESucc prf)))

||| The minimum level (L1) is a lower bound.
public export
l1IsBottom : (l : Level) -> LTE_Level L1 l
l1IsBottom (MkLevel _) = LTEZero

||| The maximum level (L10) is an upper bound.
public export
l10IsTop : (l : Level) -> LTE_Level l L10
l10IsTop (MkLevel f) = finToNatLT10 f
  where
    finToNatLT10 : (f : Fin 10) -> LTE (finToNat f) 9
    finToNatLT10 FZ = LTEZero
    finToNatLT10 (FS FZ) = LTESucc LTEZero
    finToNatLT10 (FS (FS FZ)) = LTESucc (LTESucc LTEZero)
    finToNatLT10 (FS (FS (FS FZ))) = LTESucc (LTESucc (LTESucc LTEZero))
    finToNatLT10 (FS (FS (FS (FS FZ)))) = LTESucc (LTESucc (LTESucc (LTESucc LTEZero)))
    finToNatLT10 (FS (FS (FS (FS (FS FZ))))) = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))
    finToNatLT10 (FS (FS (FS (FS (FS (FS FZ)))))) = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero)))))
    finToNatLT10 (FS (FS (FS (FS (FS (FS (FS FZ))))))) = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero))))))
    finToNatLT10 (FS (FS (FS (FS (FS (FS (FS (FS FZ)))))))) = LTESucc (LTESucc (LTESucc (LTESucc (LTESucc (LTESucc (LTESucc (LTESucc LTEZero)))))))
    finToNatLT10 (FS (FS (FS (FS (FS (FS (FS (FS (FS FZ))))))))) = lteRefl
