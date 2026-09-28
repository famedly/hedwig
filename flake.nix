# SPDX-FileCopyrightText: 2026 Famedly GmbH
#
# SPDX-License-Identifier: Apache-2.0

{
  description = "Hedwig";

  inputs = {
    famedly-engineering-standards.url = "github:famedly/engineering-standards";

    nixpkgs.follows = "famedly-engineering-standards/nixpkgs";
    flake-parts.follows = "famedly-engineering-standards/flake-parts";
  };

  outputs =
    { famedly-engineering-standards, flake-parts, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ famedly-engineering-standards.flakeModules.default ];

      systems = famedly-engineering-standards.lib.famedlySystems;

      perSystem = { config, ... }: {
        # These files are deliberate test fixtures containing dummy private
        # keys; exclude them from the detect-private-key pre-commit hook.
        prek-pre-commit.workspaces.".".exclude = "^tests/(test\\.key|dummy-service-account\\.json)$";

        # Specify a default devshell for the project; other options are
        # documented in the devshells section below.
        #
        # This is the devshell the standards assemble for *this* repository,
        # carrying the toolchain the configuration below asks for. Taking it
        # from the standards flake instead would get you the one that repository
        # uses on itself, which pins none of your tools.
        #
        # devShells.default = config.devShells.standards;

        famedly.standards = {
          # Read module documentation for further details, but most
          # likely you want one of the following:
          #
          # dart.projects."." = { };                  # Flutter: { flutter = true; }
          # rust.projects."." = { };
        };
      };
    };
}
