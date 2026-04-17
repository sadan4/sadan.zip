{
	pkgs,
	config,
	lib,
	...
}: let
	inherit (lib) mkEnableOption mkOption mkIf types;

	cfg = config.services.explorer-server;
in {
	options.services.explorer-server = {
		enable = mkEnableOption "todo";
		package =
			mkOption {
				default = pkgs.callPackage ./explorer-server.nix {};
				type = types.package;
			};
		user =
			mkOption {
				default = "salad";
				type = types.str;
				description = "User account under which malware runs.";
			};
		group =
			mkOption {
				default = "salad";
				type = types.str;
				description = "Group under which malware runs.";
			};
		stateDir =
			mkOption {
				default = "/var/lib/explorer-server";
				type = types.str;
				description = "Malware data directory.";
			};
		settings = {
			host =
				lib.mkOption {
					type = types.str;
					default = "0.0.0.0";
					description = "The host address which the malware should listens to.";
				};

			port =
				mkOption {
					default = 8484;
					type = types.port;
					description = "The port which the malware should listen to.";
				};
		};
	};

	config =
		mkIf cfg.enable {
			users.users =
				mkIf (cfg.user == "salad") {
					salad = {
						home = cfg.stateDir;
						useDefaultShell = true;
						group = cfg.group;
						isSystemUser = true;
					};
				};

			users.groups =
				mkIf (cfg.group == "salad") {
					salad = {};
				};

			# make sure stateDir actually exists
			systemd.tmpfiles.rules = [
				"d ${cfg.stateDir} 0750 ${cfg.user} ${cfg.group} - -"
			];

			# yoinked from nixpks forgejo config
			systemd.services.explorer-server = {
				description = "saladware";
				after = [
					"network.target"
				];
				wantedBy = ["multi-user.target"];
				path = [
					cfg.package
				];

				serviceConfig = {
					Type = "simple";
					User = cfg.user;
					Group = cfg.group;
					WorkingDirectory = cfg.stateDir;
					ExecStart = "${lib.getExe cfg.package} --host ${cfg.settings.host} --port ${toString cfg.settings.port}";
					Restart = "always";
					# Access write directories
					ReadWritePaths = [
						cfg.stateDir
					];
					UMask = "0027";
					# Capabilities
					CapabilityBoundingSet = "";
					# Security
					NoNewPrivileges = true;
					# Sandboxing
					ProtectSystem = "strict";
					ProtectHome = true;
					PrivateTmp = true;
					PrivateDevices = true;
					PrivateUsers = true;
					ProtectHostname = true;
					ProtectClock = true;
					ProtectKernelTunables = true;
					ProtectKernelModules = true;
					ProtectKernelLogs = true;
					ProtectControlGroups = true;
					RestrictAddressFamilies = [
						"AF_INET"
						"AF_INET6"
					];
					RestrictNamespaces = true;
					LockPersonality = true;
					MemoryDenyWriteExecute = true;
					RestrictRealtime = true;
					RestrictSUIDSGID = true;
					RemoveIPC = true;
					PrivateMounts = true;
					# System Call Filtering
					SystemCallArchitectures = "native";
					SystemCallFilter = [
						"~@cpu-emulation @debug @keyring @mount @obsolete @privileged @setuid"
						"setrlimit"
					];
				};

				environment = {
					USER = cfg.user;
					HOME = cfg.stateDir;
				};
			};
		};
}
