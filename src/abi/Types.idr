-- SPDX-License-Identifier: MPL-2.0
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
--
||| ABI Type Definitions Template
|||
||| This module defines the Application Binary Interface (ABI) for this library.
||| All type definitions include formal proofs of correctness.
|||
||| Replace TYPELL with your project name.
|||
||| @see https://idris2.readthedocs.io for Idris2 documentation

module TYPELL.ABI.Types

import Data.Bits
import Data.So
import Data.Vect
import Decidable.Equality

%default total

--------------------------------------------------------------------------------
-- Platform Detection
--------------------------------------------------------------------------------

||| Supported platforms for this ABI
public export
data Platform = Linux | Windows | MacOS | BSD | WASM

||| The platform this ABI build targets.
|||
||| Honesty note (2026-07-21): the previous definition wrapped `pure Linux`
||| in a %runElab block labelled "platform detection logic" — it detected
||| nothing and did not even compile (%language ElabReflection was never
||| enabled, so this module failed idris2 --check). It is a plain constant
||| until real per-target configuration exists.
public export
thisPlatform : Platform
thisPlatform = Linux

--------------------------------------------------------------------------------
-- Core Types
--------------------------------------------------------------------------------

||| Result codes for FFI operations
||| Use C-compatible integers for cross-language compatibility
public export
data Result : Type where
  ||| Operation succeeded
  Ok : Result
  ||| Generic error
  Error : Result
  ||| Invalid parameter provided
  InvalidParam : Result
  ||| Out of memory
  OutOfMemory : Result
  ||| Null pointer encountered
  NullPointer : Result

||| Convert Result to C integer
public export
resultToInt : Result -> Bits32
resultToInt Ok = 0
resultToInt Error = 1
resultToInt InvalidParam = 2
resultToInt OutOfMemory = 3
resultToInt NullPointer = 4

||| Results are decidably equal
public export
DecEq Result where
  decEq Ok Ok = Yes Refl
  decEq Error Error = Yes Refl
  decEq InvalidParam InvalidParam = Yes Refl
  decEq OutOfMemory OutOfMemory = Yes Refl
  decEq NullPointer NullPointer = Yes Refl
  decEq Ok Error = No (\case Refl impossible)
  decEq Ok InvalidParam = No (\case Refl impossible)
  decEq Ok OutOfMemory = No (\case Refl impossible)
  decEq Ok NullPointer = No (\case Refl impossible)
  decEq Error Ok = No (\case Refl impossible)
  decEq Error InvalidParam = No (\case Refl impossible)
  decEq Error OutOfMemory = No (\case Refl impossible)
  decEq Error NullPointer = No (\case Refl impossible)
  decEq InvalidParam Ok = No (\case Refl impossible)
  decEq InvalidParam Error = No (\case Refl impossible)
  decEq InvalidParam OutOfMemory = No (\case Refl impossible)
  decEq InvalidParam NullPointer = No (\case Refl impossible)
  decEq OutOfMemory Ok = No (\case Refl impossible)
  decEq OutOfMemory Error = No (\case Refl impossible)
  decEq OutOfMemory InvalidParam = No (\case Refl impossible)
  decEq OutOfMemory NullPointer = No (\case Refl impossible)
  decEq NullPointer Ok = No (\case Refl impossible)
  decEq NullPointer Error = No (\case Refl impossible)
  decEq NullPointer InvalidParam = No (\case Refl impossible)
  decEq NullPointer OutOfMemory = No (\case Refl impossible)

--------------------------------------------------------------------------------
-- Opaque Handles
--------------------------------------------------------------------------------

||| Opaque handle type for FFI
||| Prevents direct construction, enforces creation through safe API
public export
data Handle : Type where
  MkHandle : (ptr : Bits64) -> {auto 0 nonNull : So (ptr /= 0)} -> Handle

||| Safely create a handle from a pointer value.
||| Returns Nothing if pointer is null.
||| Uses Data.So.choose so the non-null proof is DECIDED, not assumed —
||| the previous `createHandle ptr = Just (MkHandle ptr)` clause could
||| never satisfy the auto So proof (clause order does not refine ptr).
public export
createHandle : Bits64 -> Maybe Handle
createHandle ptr = case choose (ptr /= 0) of
  Left prf => Just (MkHandle ptr {nonNull = prf})
  Right _  => Nothing

||| Extract pointer value from handle
public export
handlePtr : Handle -> Bits64
handlePtr (MkHandle ptr) = ptr

--------------------------------------------------------------------------------
-- Platform-Specific Types
--------------------------------------------------------------------------------

||| C int size varies by platform
public export
CInt : Platform -> Type
CInt Linux = Bits32
CInt Windows = Bits32
CInt MacOS = Bits32
CInt BSD = Bits32
CInt WASM = Bits32

||| C size_t varies by platform
public export
CSize : Platform -> Type
CSize Linux = Bits64
CSize Windows = Bits64
CSize MacOS = Bits64
CSize BSD = Bits64
CSize WASM = Bits32

||| C pointer size varies by platform
public export
ptrSize : Platform -> Nat
ptrSize Linux = 64
ptrSize Windows = 64
ptrSize MacOS = 64
ptrSize BSD = 64
ptrSize WASM = 32

||| Pointer representation for platform.
||| (`Bits (ptrSize p)` in the template was ill-typed: Data.Bits.Bits is
||| an interface, not a `Nat -> Type` family.)
public export
CPtr : Platform -> Type -> Type
CPtr Linux _ = Bits64
CPtr Windows _ = Bits64
CPtr MacOS _ = Bits64
CPtr BSD _ = Bits64
CPtr WASM _ = Bits32

--------------------------------------------------------------------------------
-- Memory Layout
--------------------------------------------------------------------------------
-- Honesty note (2026-07-21): the template shipped `cSizeOf : Platform ->
-- Type -> Nat` pattern-matching on Type (not possible in Idris 2), plus
-- HasSize/HasAlignment "proofs" whose sole constructor proved ANY type
-- has ANY size — vacuous by construction. Replaced with a closed universe
-- of C types, so sizes are computable and the lemmas below are real
-- definitional equalities.

||| The closed universe of C types this ABI can describe.
public export
data CTy : Type where
  CTInt    : CTy
  CTSize   : CTy
  CTPtr    : CTy
  CTBits32 : CTy
  CTBits64 : CTy
  CTDouble : CTy

||| Pointer width in BYTES per platform. Stated directly (not as
||| ptrSize/8) so the layout lemmas below hold definitionally — Nat
||| division does not reduce by Refl.
public export
ptrBytes : Platform -> Nat
ptrBytes Linux = 8
ptrBytes Windows = 8
ptrBytes MacOS = 8
ptrBytes BSD = 8
ptrBytes WASM = 4

||| Size in bytes of each C type on a platform.
public export
cSizeOf : (p : Platform) -> CTy -> Nat
cSizeOf p CTInt    = 4
cSizeOf p CTSize   = ptrBytes p
cSizeOf p CTPtr    = ptrBytes p
cSizeOf p CTBits32 = 4
cSizeOf p CTBits64 = 8
cSizeOf p CTDouble = 8

||| Alignment in bytes of each C type on a platform.
||| For these scalar types alignment equals size on all supported ABIs.
public export
cAlignOf : (p : Platform) -> CTy -> Nat
cAlignOf p t = cSizeOf p t

||| Real layout lemmas — definitional equalities the checker verifies,
||| replacing the former vacuous HasSize/HasAlignment.
public export
cIntIs4Bytes : (p : Platform) -> cSizeOf p CTInt = 4
cIntIs4Bytes p = Refl

public export
cBits64Is8Bytes : (p : Platform) -> cSizeOf p CTBits64 = 8
cBits64Is8Bytes p = Refl

public export
ptrIs8BytesOnLinux : cSizeOf Linux CTPtr = 8
ptrIs8BytesOnLinux = Refl

public export
ptrIs4BytesOnWASM : cSizeOf WASM CTPtr = 4
ptrIs4BytesOnWASM = Refl

--------------------------------------------------------------------------------
-- FFI Declarations
--------------------------------------------------------------------------------

||| Declare external C functions
||| These will be implemented in Zig FFI (ffi/zig/)
namespace Foreign

  ||| External function example
  export
  %foreign "C:example_function, libexample"
  prim__exampleFunction : Bits64 -> PrimIO Bits32

  ||| Safe wrapper around FFI function
  export
  exampleFunction : Handle -> IO (Either Result Bits32)
  exampleFunction h = do
    result <- primIO (prim__exampleFunction (handlePtr h))
    pure (Right result)
