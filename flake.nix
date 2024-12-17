{
  description = "A basic flake with a shell";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  inputs.systems.url = "github:nix-systems/default";
  inputs.flake-utils = {
    url = "github:numtide/flake-utils";
    inputs.systems.follows = "systems";
  };

  outputs =
    { nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        postgresConf = ''
          log_min_messages = warning
          log_min_error_statement = error
          log_min_duration_statement = 100  # ms
          log_connections = on
          log_disconnections = on
          log_duration = on
          log_timezone = 'UTC'
          log_statement = 'all'
          log_directory = 'logs'
          log_filename = 'postgresql-%Y-%m-%d_%H%M%S.log'
          logging_collector = on
          log_min_error_statement = error
        '';
        postgresStart = ''
          echo "Starting postgres server"
          pg_ctl start
        '';
        postgresStop = ''
          echo "Stopping postgres server"
          pg_ctl stop
        '';
        env = ''
          #!/usr/bin/env zsh

          DATABASE_URL=postgres://dev@localhost:5432/postgres

        '';
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            postgresql_16_jit
            sqlx-cli
          ];

          shellHook = ''
            export FLAKE_ROOT="$(git rev-parse --show-toplevel)"

            echo "${env}" > $FLAKE_ROOT/.env
            set -a; source .env; set +a

            mkdir -p $FLAKE_ROOT/postgres
            echo "$FLAKE_ROOT"
            export PGDATA="$FLAKE_ROOT/postgres/.pg"
            export PGHOST=localhost
            export PGPORT=5432
            export PGUSER=dev
            export PGDATABASE=postgres

            echo "Initializing database"
            [ ! -d $PGDATA ] && PGHOST="$PGDATA" pg_ctl initdb -o "-U $PGUSER" && cat "${postgresConf}" >> $PGDATA/postgresql.conf

            echo "Writing postgres stop and start scripts"
            echo "${postgresStart}" > $FLAKE_ROOT/postgres/start
            chmod +x $FLAKE_ROOT/postgres/start
            echo "${postgresStop}" > $FLAKE_ROOT/postgres/stop
            chmod +x $FLAKE_ROOT/postgres/stop
          '';
        };
      }
    );
}
