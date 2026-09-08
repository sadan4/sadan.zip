import { ExtensionContext } from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from "vscode-languageclient/node";
import { Settings } from "./Settings";
import { join } from "node:path";

let client: LanguageClient | undefined;

export async function activate(cx: ExtensionContext): Promise<void> {
    if (client) { 
        throw new Error("Vencord Companion LSP client is already active");
    }
    client = mkClient(cx);
    client.start();
}

function mkClient(cx: ExtensionContext): LanguageClient { 
    const serverOptions: ServerOptions = {
        command: resolveServerBinary(cx),
        transport: TransportKind.stdio,
    };
    const clientOptions: LanguageClientOptions = {};
    return new LanguageClient("vencord-companion-client", "Vencord Companion", serverOptions, clientOptions);
}

export async function deactivate(): Promise<void> {
    await client?.stop();
    client = undefined;
}

function resolveServerBinary(cx: ExtensionContext): string {
    const env = process.env.COMPANION_LSP_BIN;
    if (env) return env;

    const cfgLspPath = Settings.lspPath;
    if (cfgLspPath) return cfgLspPath;

    const bundled = join(
        cx.extensionPath,
        "bin",
        process.platform === "win32" ? "companion_lsp.exe" : "companion_lsp",
    );
    return bundled;
}