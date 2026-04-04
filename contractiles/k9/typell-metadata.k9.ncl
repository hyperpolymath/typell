K9!
# SPDX-License-Identifier: PMPL-1.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
# K9 Kennel-level guard: typell project metadata and build constraints
# Security Level: Kennel (data-only, no execution)
# No signature required

{
  pedigree = {
    schema_version = "1.0.0",
    component_type = "project-metadata",
    security = {
      leash = 'Kennel,
      trust_level = "data-only",
      allow_network = false,
      allow_filesystem_write = false,
      allow_subprocess = false,
    },
    metadata = {
      name = "typell-metadata",
      version = "0.1.0",
      description = "TypeLL universal type system library — project metadata and invariants",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },
  },

  # Project configuration
  config = {
    # Workspace crates that must be present
    required_crates = [
      "typell-core",
      "typell-eclexia",
      "typell-affinescript",
      "typell-ephapax",
      "typell-wokelang",
      "typell-tangle",
      "typell-betlang",
      "typell-mylang",
      "typell-oblibeny",
    ],

    # Type system levels that must be covered by tests
    type_system_levels = [
      "dependent",
      "linear",
      "session",
      "qtt",
      "effects",
      "modal",
      "dimensional",
      "proof",
      "epistemic",
      "tropical",
    ],

    # Minimum test counts per crate
    min_tests = {
      typell-core = 50,
    },

    # Build configuration
    build = {
      edition = "2021",
      msrv = "1.75",
      features = ["full"],
    },

    # Language policy compliance
    banned = {
      languages = ["python", "typescript", "javascript", "go"],
      files = ["Makefile", "Dockerfile"],
    },
  },

  # RSR compliance declarations
  rsr = {
    explainme = true,
    ai_manifest = true,
    machine_readable = true,
    spdx_headers_required = true,
    idris2_abi = true,
  },
}
