//! Custom JSON-RPC surface that lives *next to* standard LSP. Editors that
//! want the full VencordCompanion feature set implement a small shim that
//! handles these requests/notifications. Editors that don't get only standard
//! LSP features (hover/definition/references/diagnostics/code lenses).

// ---- Commands the client may send via `workspace/executeCommand` ----------

pub const CMD_EXTRACT_MODULE: &str = "vencord.extractModule";
pub const CMD_EXTRACT_FIND: &str = "vencord.extractFind";
pub const CMD_DIFF_MODULE: &str = "vencord.diffModule";
pub const CMD_DISABLE_PLUGIN: &str = "vencord.disablePlugin";
pub const CMD_TEST_PATCH: &str = "vencord.testPatch";
pub const CMD_TEST_FIND: &str = "vencord.testFind";
pub const CMD_OPEN_PATCH_HELPER: &str = "vencord.openPatchHelper";
pub const CMD_WEBPACK_I18N_HOVER_COPY: &str = "vencord.webpackI18nHoverCopy";
pub const CMD_DOWNLOAD_MODULE_CACHE: &str = "vencord.downloadModuleCache";
pub const CMD_PURGE_MODULE_CACHE: &str = "vencord.purgeModuleCache";

pub const ALL_COMMAND_IDS: &[&str] = &[
	CMD_EXTRACT_MODULE,
	CMD_EXTRACT_FIND,
	CMD_DIFF_MODULE,
	CMD_DISABLE_PLUGIN,
	CMD_TEST_PATCH,
	CMD_TEST_FIND,
	CMD_OPEN_PATCH_HELPER,
	CMD_WEBPACK_I18N_HOVER_COPY,
	CMD_DOWNLOAD_MODULE_CACHE,
	CMD_PURGE_MODULE_CACHE,
];

// ---- Server -> Client requests (custom methods) ---------------------------

pub const QUICK_PICK_METHOD: &str = "vencord/quickPick";
pub const SHOW_DIFF_METHOD: &str = "vencord/showDiff";
pub const PATCH_HELPER_OPEN_METHOD: &str = "vencord/patchHelper/open";
pub const PATCH_HELPER_UPDATE_METHOD: &str = "vencord/patchHelper/update";
pub const PATCH_HELPER_CLOSE_METHOD: &str = "vencord/patchHelper/close";

// ---- Client -> Server requests (custom methods) ---------------------------

/// Used by the editor shim to deliver the user's QuickPick selection back to
/// the server (nonce-correlated with the original `vencord/quickPick` request).
pub const QUICK_PICK_RESPONSE_METHOD: &str = "vencord/quickPick/response";
