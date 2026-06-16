import {
    commands,
    ConfigurationTarget,
    EventEmitter,
    ExtensionContext,
    Position,
    QuickPickItem,
    Range as VscRange,
    TabInputText,
    TextDocumentContentProvider,
    TextEditorRevealType,
    Uri,
    ViewColumn,
    window,
    workspace,
} from "vscode";
import * as path from "node:path";
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from "vscode-languageclient/node";

// All Vencord-specific UI flows go through these custom JSON-RPC methods,
// declared in crates/companion_lsp/src/vencord_ext.rs. The server is the
// source of truth; this file only mirrors method names.
const QUICK_PICK_METHOD = "vencord/quickPick";
const QUICK_PICK_RESPONSE_METHOD = "vencord/quickPick/response";
const SHOW_DIFF_METHOD = "vencord/showDiff";
const PATCH_HELPER_OPEN_METHOD = "vencord/patchHelper/open";
const PATCH_HELPER_CLOSE_METHOD = "vencord/patchHelper/close";
const PATCH_HELPER_UPDATE_METHOD = "vencord/patchHelper/update";

// The server advertises its `vencord.*` command IDs in
// ServerCapabilities.executeCommandProvider.commands, and vscode-languageclient
// auto-registers a VSCode command for each that forwards via
// workspace/executeCommand. Do NOT also register them here, or activation
// fails with "command 'vencord.extractModule' already exists".

// Legacy command palette entries kept for backward compatibility.
const LEGACY_COMMAND_ALIASES: { vscode: string; server: string }[] = [
    { vscode: "vencord-companion.diffModule",        server: "vencord.diffModule" },
    { vscode: "vencord-companion.diffModuleSearch",  server: "vencord.diffModule" },
    { vscode: "vencord-companion.extract",           server: "vencord.extractModule" },
    { vscode: "vencord-companion.extractSearch",     server: "vencord.extractModule" },
    { vscode: "vencord-companion.extractFind",       server: "vencord.extractFind" },
    { vscode: "vencord-companion.disablePlugin",     server: "vencord.disablePlugin" },
    { vscode: "vencord-companion.testPatch",         server: "vencord.testPatch" },
    { vscode: "vencord-companion.testFind",          server: "vencord.testFind" },
    { vscode: "vencord-companion.openPatchHelper",   server: "vencord.openPatchHelper" },
];

let client: LanguageClient | undefined;
// Stashed at activation so the "Set Log Level" command can rebuild the client
// (the log level is baked into the server env at spawn time, so changing it
// means restarting the process).
let extensionContext: ExtensionContext | undefined;

// Mirrors the levels accepted by the server's EnvFilter (COMPANION_LSP_LOG),
// set up in crates/companion_lsp/src/main.rs.
const LOG_LEVELS = ["trace", "debug", "info", "warn", "error", "off"] as const;
type LogLevel = typeof LOG_LEVELS[number];

function getConfiguredLogLevel(): LogLevel {
    const cfg = workspace.getConfiguration("vencord-user-companion").get<string>("logLevel");
    return (LOG_LEVELS as readonly string[]).includes(cfg ?? "") ? cfg as LogLevel : "info";
}

function createClient(context: ExtensionContext): LanguageClient {
    const serverPath = resolveServerBinary(context);
    const logLevel = getConfiguredLogLevel();
    // For an Executable server, vscode-languageclient passes `options.env`
    // straight to child_process.spawn, which REPLACES the environment rather
    // than extending it. Spread process.env so the server keeps PATH/HOME/etc.
    // (the discord bridge needs them) and only override the log level.
    const env = { ...process.env, COMPANION_LSP_LOG: logLevel };
    const serverOptions: ServerOptions = {
        run:   { command: serverPath, transport: TransportKind.stdio, options: { env } },
        debug: { command: serverPath, transport: TransportKind.stdio, options: { env } },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: "file", language: "typescript" },
            { scheme: "file", language: "typescriptreact" },
            { scheme: "file", language: "javascript" },
            { scheme: "file", language: "javascriptreact" },
            // Patch Helper virtual documents (vencord-patchhelper:/session/*.js)
            // so intl hover works on the patched webpack module shown there.
            { scheme: "vencord-patchhelper", language: "javascript" },
        ],
        synchronize: {
            fileEvents: workspace.createFileSystemWatcher("**/*.{ts,tsx,js,jsx}"),
        },
    };

    return new LanguageClient(
        "companionLsp",
        "Vencord Companion LSP",
        serverOptions,
        clientOptions,
    );
}

export async function activate(context: ExtensionContext): Promise<void> {
    extensionContext = context;
    client = createClient(context);

    // Command forwarders and the patch helper reference the module-level
    // `client`, so they survive a restart and only need registering once.
    registerCommandForwarders(context);
    registerPatchHelper(context);
    context.subscriptions.push(
        commands.registerCommand("vencord-companion.setLogLevel", setLogLevel),
    );

    await client.start();
    registerCustomRequests();
}

export async function deactivate(): Promise<void> {
    if (client) {
        await client.stop();
    }
}

// ---------------------------------------------------------------------------
// Log level: pick a level, persist it, restart the server with it applied
// ---------------------------------------------------------------------------

async function setLogLevel(): Promise<void> {
    const current = getConfiguredLogLevel();
    const picked = await window.showQuickPick(
        LOG_LEVELS.map((level) => ({
            label:       level,
            description: level === current ? "(current)" : undefined,
        })),
        { placeHolder: "Select the companion_lsp log level" },
    );
    if (!picked || picked.label === current) return;

    await workspace.getConfiguration("vencord-user-companion")
        .update("logLevel", picked.label, ConfigurationTarget.Global);
    await restartClient();
    window.showInformationMessage(`Vencord Companion: log level set to "${picked.label}".`);
}

async function restartClient(): Promise<void> {
    if (!extensionContext) return;
    if (client) {
        await client.stop();
    }
    client = createClient(extensionContext);
    await client.start();
    registerCustomRequests();
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

function resolveServerBinary(context: ExtensionContext): string {
    // 1. Env var override for development.
    const env = process.env.COMPANION_LSP_BIN;
    if (env) return env;

    // 2. User-configured path.
    const cfg = workspace.getConfiguration("vencord-user-companion").get<string>("lspPath");
    if (cfg) return cfg;

    // 3. Binary bundled with the extension (release path).
    const bundled = path.join(
        context.extensionPath,
        "bin",
        process.platform === "win32" ? "companion_lsp.exe" : "companion_lsp",
    );
    return bundled;
}

// ---------------------------------------------------------------------------
// Forward VSCode commands -> server workspace/executeCommand
// ---------------------------------------------------------------------------

function registerCommandForwarders(context: ExtensionContext): void {
    const forward = (cmd: string) => (...args: unknown[]) =>
        client?.sendRequest("workspace/executeCommand", {
            command:   cmd,
            arguments: args,
        });

    for (const { vscode: vsCmd, server: srvCmd } of LEGACY_COMMAND_ALIASES) {
        context.subscriptions.push(commands.registerCommand(vsCmd, forward(srvCmd)));
    }
}

// ---------------------------------------------------------------------------
// vencord/quickPick: server asks editor to display a QuickPick
// ---------------------------------------------------------------------------

interface QuickPickRequest {
    nonce:          string;
    items:          string[];
    placeholder?:   string;
    allowFreeText?: boolean;
}

async function handleQuickPick(req: QuickPickRequest): Promise<void> {
    const qp = window.createQuickPick();
    qp.placeholder    = req.placeholder ?? "";
    qp.canSelectMany  = false;
    const items: QuickPickItem[] = [
        ...(req.allowFreeText ? [{ label: "", alwaysShow: false }] : []),
        { label: "", kind: -1 as const },
        ...req.items.map((label) => ({ label })),
    ];
    qp.items = items;

    if (req.allowFreeText) {
        qp.onDidChangeValue(() => {
            if (!req.items.includes(qp.value)) {
                items[0].label      = qp.value;
                items[0].alwaysShow = true;
            } else {
                items[0].alwaysShow = false;
            }
            qp.items = items;
        });
    }

    const selected = await new Promise<string | null>((resolve) => {
        qp.onDidAccept(() => {
            const v = qp.value || qp.selectedItems[0]?.label || null;
            qp.dispose();
            resolve(v);
        });
        qp.onDidHide(() => resolve(null));
        qp.show();
    });

    client?.sendRequest(QUICK_PICK_RESPONSE_METHOD, { nonce: req.nonce, selected });
}

// ---------------------------------------------------------------------------
// vencord/showDiff
// ---------------------------------------------------------------------------

interface ShowDiffRequest {
    left:  { uri: string; content?: string };
    right: { uri: string; content?: string };
    title: string;
}

async function handleShowDiff(req: ShowDiffRequest): Promise<void> {
    const left  = Uri.parse(req.left.uri);
    const right = Uri.parse(req.right.uri);
    await commands.executeCommand("vscode.diff", left, right, req.title);
}

// ---------------------------------------------------------------------------
// Patch helper: virtual document scheme backed by server-provided content
// ---------------------------------------------------------------------------

interface PatchHelperOpen {
    sourceUri:     string;
    patchId:       string;
    moduleContent: string;
}

interface PatchHelperUpdate {
    sourceUri:     string;
    patchId:       string;
    moduleContent: string;
    // Server-computed 0-based line range covering the difference between the
    // previous and new patched module. Null on the first update (no previous
    // content to diff against) or when nothing changed.
    revealRange?:  { startLine: number; endLine: number } | null;
}

interface PatchHelperClose {
    sourceUri: string;
    patchId:   string;
}

class PatchHelperProvider implements TextDocumentContentProvider {
    private readonly contents = new Map<string, string>();
    private readonly _onDidChange = new EventEmitter<Uri>();
    readonly onDidChange = this._onDidChange.event;

    open(patchId: string, content: string): Uri {
        const uri = Uri.parse(`vencord-patchhelper:/session/${patchId}.js`);
        this.contents.set(uri.toString(), content);
        return uri;
    }

    update(patchId: string, content: string): void {
        const uri = Uri.parse(`vencord-patchhelper:/session/${patchId}.js`);
        this.contents.set(uri.toString(), content);
        this._onDidChange.fire(uri);
    }

    uriFor(patchId: string): Uri {
        return Uri.parse(`vencord-patchhelper:/session/${patchId}.js`);
    }

    drop(patchId: string): void {
        this.contents.delete(this.uriFor(patchId).toString());
    }

    provideTextDocumentContent(uri: Uri): string {
        return this.contents.get(uri.toString()) ?? "";
    }
}

let patchHelperProvider: PatchHelperProvider | undefined;
// sourceUri (string) → patchId. Lets the tab-close listener find the
// patched view to close when the user shuts the plugin source tab — VSCode
// often holds the underlying TextDocument in memory after the tab is gone,
// so `workspace.onDidCloseTextDocument` (and the LSP `didClose` it relays)
// fires too late to be the only signal we react to.
const sourceUriToPatchId = new Map<string, string>();

function registerPatchHelper(context: ExtensionContext): void {
    patchHelperProvider = new PatchHelperProvider();
    context.subscriptions.push(
        workspace.registerTextDocumentContentProvider(
            "vencord-patchhelper",
            patchHelperProvider,
        ),
        window.tabGroups.onDidChangeTabs(onTabsChanged),
    );
}

async function handlePatchHelperOpen(req: PatchHelperOpen): Promise<void> {
    if (!patchHelperProvider) return;
    sourceUriToPatchId.set(req.sourceUri, req.patchId);
    const uri = patchHelperProvider.open(req.patchId, req.moduleContent);
    const doc = await workspace.openTextDocument(uri);
    // Beside, not preview — matches the legacy PatchHelper behaviour so the
    // patched view sits next to the plugin source instead of replacing it.
    await window.showTextDocument(doc, {
        preview: false,
        viewColumn: ViewColumn.Beside,
    });
}

function handlePatchHelperUpdate(req: PatchHelperUpdate): void {
    patchHelperProvider?.update(req.patchId, req.moduleContent);
    if (!req.revealRange) return;
    // VSCode applies the content-provider update in a microtask after `fire`,
    // so defer the reveal one tick so the editor isn't scrolled before the
    // new line layout exists.
    const { startLine, endLine } = req.revealRange;
    const targetUri = patchHelperProvider?.uriFor(req.patchId).toString();
    if (!targetUri) return;
    queueMicrotask(() => {
        const editor = window.visibleTextEditors.find(
            (e) => e.document.uri.toString() === targetUri,
        );
        if (!editor) return;
        const range = new VscRange(
            new Position(startLine, 0),
            new Position(endLine, 0),
        );
        editor.revealRange(range, TextEditorRevealType.InCenter);
    });
}

async function handlePatchHelperClose(req: PatchHelperClose): Promise<void> {
    sourceUriToPatchId.delete(req.sourceUri);
    await closePatchHelperByPatchId(req.patchId);
}

async function closePatchHelperByPatchId(patchId: string): Promise<void> {
    if (!patchHelperProvider) return;
    const targetUri = patchHelperProvider.uriFor(patchId).toString();
    // Walk every tab group looking for the matching virtual document tab.
    // There's only ever one per patchId since uriFor is deterministic, but
    // VSCode permits the same tab in multiple groups, so close them all.
    const tabs = window.tabGroups.all
        .flatMap((g) => g.tabs)
        .filter((tab) =>
            tab.input instanceof TabInputText
            && tab.input.uri.toString() === targetUri,
        );
    if (tabs.length > 0) {
        await window.tabGroups.close(tabs);
    }
    patchHelperProvider.drop(patchId);
}

async function onTabsChanged(e: { closed: readonly { input: unknown }[] }): Promise<void> {
    for (const tab of e.closed) {
        if (!(tab.input instanceof TabInputText)) continue;
        const closedUri = tab.input.uri.toString();

        // Source tab closed: close the linked patch helper tab.
        const patchId = sourceUriToPatchId.get(closedUri);
        if (patchId !== undefined) {
            sourceUriToPatchId.delete(closedUri);
            await closePatchHelperByPatchId(patchId);
            continue;
        }

        // Patch helper tab closed by the user: drop our cache and the
        // sourceUri → patchId mapping for it.
        if (tab.input.uri.scheme === "vencord-patchhelper") {
            const id = patchIdFromHelperUri(tab.input.uri);
            if (id !== undefined) {
                patchHelperProvider?.drop(id);
                for (const [src, pid] of sourceUriToPatchId) {
                    if (pid === id) sourceUriToPatchId.delete(src);
                }
            }
        }
    }
}

function patchIdFromHelperUri(uri: Uri): string | undefined {
    // Mirrors PatchHelperProvider.uriFor: `vencord-patchhelper:/session/<id>.js`.
    const match = /^\/session\/(.+)\.js$/.exec(uri.path);
    return match ? match[1] : undefined;
}

// ---------------------------------------------------------------------------
// Wire all custom requests onto the client
// ---------------------------------------------------------------------------

function registerCustomRequests(): void {
    if (!client) return;
    client.onRequest(QUICK_PICK_METHOD, handleQuickPick);
    client.onRequest(SHOW_DIFF_METHOD, handleShowDiff);
    client.onRequest(PATCH_HELPER_OPEN_METHOD, handlePatchHelperOpen);
    client.onNotification(PATCH_HELPER_UPDATE_METHOD, handlePatchHelperUpdate);
    client.onNotification(PATCH_HELPER_CLOSE_METHOD, handlePatchHelperClose);
}
