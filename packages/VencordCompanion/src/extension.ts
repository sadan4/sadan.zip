import { ExtensionContext, QuickPickItem, window as vsWindow } from "vscode";
import { ErrorCodes, LanguageClient, LanguageClientOptions, ResponseError, ServerOptions, TransportKind } from "vscode-languageclient/node";
import { Settings } from "./Settings";
import { join } from "node:path";
import * as z from "zod";

let client: LanguageClient | undefined;

const QUICK_PICK_METHOD = "$/vencord-companion/quick_pick";

const QuickPickRequest = z.object({
    items: z.array(z.string()),
    placeholder: z.string().optional(),
    allowFreeText: z.boolean().optional(),
});

type QuickPickRequest = z.infer<typeof QuickPickRequest>;

class QuickPickError extends ResponseError { 
    override name = "QuickPickError";
    constructor(msg: string, cause?: Error, code: number = ErrorCodes.UnknownErrorCode) { 
        super(ErrorCodes.UnknownErrorCode, msg)
        if (cause) { 
            this.cause = cause;
        }
    }
}


export async function activate(cx: ExtensionContext): Promise<void> {
    if (client) { 
        throw new Error("Vencord Companion LSP client is already active");
    }
    const c = client = mkClient(cx);
    client.onRequest(QUICK_PICK_METHOD, async (req: unknown) => { 
        try {
            const parsed = QuickPickRequest.parse(req);
            return await doQuickPick(parsed);
        } catch (parseErr: any) { 
            let toThrow;
            if (parseErr instanceof z.ZodError) {
                toThrow = new QuickPickError("Invalid QuickPickRequest: Failed to parse parameters", parseErr, ErrorCodes.InvalidParams);
            } else if (parseErr instanceof QuickPickError) {
                toThrow = parseErr;
            } else { 
                toThrow = new QuickPickError("Invalid QuickPickRequest: Unknown error", parseErr);
            }
            c.error(`Failed to handle QuickPickRequest`, toThrow);
            throw toThrow;
        }
    })
    client.start();
}

function doQuickPick(req: QuickPickRequest): Promise<string | undefined> { 
    const qp = vsWindow.createQuickPick();
    const { promise, resolve, reject } = Promise.withResolvers<string | undefined>();
    const baseItems = req.items.map((label) => ({ label }));
    // VSCode quick pick don't support free text input
    // so for a hacky workaround, create and update an
    // extra item with the contents of the current input value
    if (req.allowFreeText) { 
        qp.items = [
            {
                label: "",
            },
            ...baseItems,
        ];
        qp.onDidChangeValue((label) => { 
            qp.items = [
                { label },
                ...baseItems,
            ];
        })
    }

    qp.onDidAccept(() => { 
        const items = qp.selectedItems;
        if (!items.length) {
            reject(new QuickPickError("No Item Selected"));
        } else if (items.length > 1) {
            reject(new QuickPickError("Multiple Items Selected"));
        } else { 
            resolve(items[0].label);
        }
        qp.dispose();
    })
    qp.onDidHide(() => { 
        resolve(undefined);
        qp.dispose();
    })
    qp.show();
    return promise;
}

function mkClient(cx: ExtensionContext): LanguageClient { 
    const serverOptions: ServerOptions = {
        command: resolveServerBinary(cx),
		transport: TransportKind.stdio,
		options: {
			env: {}
		}
	};
	const logLevel = Settings.logLevel;
	if (logLevel) { 
		serverOptions.options!.env.COMPANION_LSP_LOG = logLevel;
	}
	const clientOptions: LanguageClientOptions = {
		documentSelector: [
			...["typescript", "javascript", "typescriptreact", "javascriptreact"]
				.map((language) => ({
					scheme: "file",
					language
				})),
			{ scheme: "vencord-companion" }
		],
	};
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
