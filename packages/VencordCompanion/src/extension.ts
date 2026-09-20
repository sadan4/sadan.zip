import * as vs from "vscode";
import {
    CancellationToken,
    commands,
    env,
    Event,
    ExtensionContext,
    Hover,
    MarkdownString,
    ProviderResult,
    QuickPickItem,
    TextDocumentContentProvider,
    Uri,
    window as vsWindow
} from "vscode";
import {
    ErrorCodes,
    State,
    LanguageClient,
    LanguageClientOptions,
    ResponseError,
    ServerOptions,
    TransportKind,
} from "vscode-languageclient/node";
import { Settings } from "./Settings";
import { join } from "node:path";
import * as z from "zod";

let client: LanguageClient | undefined;

const SERVER_NAME = "vencord-companion";

const QUICK_PICK_METHOD = `$/${SERVER_NAME}/quick_pick`;

const EPHEMERAL_DID_CHANGE_NOTI = `$/${SERVER_NAME}/ephemera/didChange`;

const EPHEMERAL_QUERY_METHOD = `$/${SERVER_NAME}/ephemera/queryDoc`;

/**
 * Not contributed in package.json on purpose:
 * `contributes.commands` is generated from the server's `CMD_MAP` by
 * `cargo xtask gen ext-commands`, and this command is handled entirely by the
 * client, so it would be clobbered (and it has no useful palette entry anyway).
 */
const COPY_COMMAND = "vencord-companion.copy";

const QuickPickRequest = z.object({
    items: z.array(z.string()),
    placeholder: z.string().optional(),
    allowFreeText: z.boolean().optional(),
});

const EphemeralChange = z.object({
    /**
     * the URI of the document that changed
     */
    uri: z.url(),
    /**
     * if present, the content of the document has changed
     */
    content: z.string().nullish(),
    /**
     * if true, the document has been closed/"deleted"
     */
    deleted: z.boolean().nullish(),
});

const EphemeralQuery = z.object({
    /**
     * the URI of the document to query
     */
    uri: z.url(),
});

const EphemeralDocument = z.object({
    /**
     * the URI of the document
     */
    uri: z.url(),
    /**
     * the content of the document
     */
    content: z.string(),
});

const EphemeralQueryResponse = z.object({
    doc: EphemeralDocument.nullish(),
});


type EphemeralQuery = z.infer<typeof EphemeralQuery>;

type EphemeralDocument = z.infer<typeof EphemeralDocument>;

type EphemeralQueryResponse = z.infer<typeof EphemeralQueryResponse>;

type EphemeralChange = z.infer<typeof EphemeralChange>;

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
    cx.subscriptions.push(commands.registerCommand(COPY_COMMAND, doCopy));
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
    handleEphemeralDocuments(cx, c);
    client.start();
}

async function handleEphemeralDocuments(cx: ExtensionContext, c: LanguageClient) { 
    c.onNotification(EPHEMERAL_DID_CHANGE_NOTI, (change: unknown) => {
        try {
            const parsed = EphemeralChange.parse(change);
            handleEphemeralChange(parsed);
        } catch (e: any) {
            if (e instanceof z.ZodError) {
                let err = z.prettifyError(e);
                c.error(`Failed to handle EphemeralChange notification: Invalid parameters`, `\n${err}`);
            } else {
                c.error(`Failed to handle EphemeralChange notification: ${e}`)
            }
        }
    });
    let ephemeralEvent = new vs.EventEmitter<Uri>();
    /**
     * What the server last told us each document holds, keyed by
     * {@link Uri.toString}, not {@link EphemeralChange.uri}
     *
     * undefined means the document is closed/deleted
     */
    let docs = new Map<string, string | undefined>();
    function handleEphemeralChange(change: EphemeralChange) {
        let uri = Uri.parse(change.uri);
        let key = uri.toString();
        using _ = defer(() => ephemeralEvent.fire(uri));
        if (change.deleted) {
            if (!docs.has(key)) {
                c.warn(`Received EphemeralChange for ${key} with deleted=true, but no document was found`);
            } else if (docs.get(key) === undefined) {
                c.warn(`Received EphemeralChange for ${key} with deleted=true, but document was already deleted`);
            }
            docs.set(key, undefined);
            return;
        }
        if (change.content != null) {
            c.debug(`Updating ephemeral document ${key} with new content of length ${change.content.length}`);
            docs.set(key, change.content);
        }
    }
    async function provideEphemeralDocument(uri: Uri, token: CancellationToken): Promise<string | undefined> {
        let key = uri.toString();
        if (docs.has(key)) {
            return docs.get(key) ?? "";
        }
        let raw = await c.sendRequest(
            EPHEMERAL_QUERY_METHOD,
            {
                uri: key
            } satisfies EphemeralQuery,
            token
        );
        let res = EphemeralQueryResponse.parse(raw);
        let content = res.doc?.content;
        if (content != null) { 
            handleEphemeralChange({
                uri: key,
                content,
            })
        }
        return content;
    }
    c.onDidChangeState((e) => {
        if (e.oldState === State.Running) {
            assert(e.newState === State.Stopped, "Client should only transition from Running to Stopped");
            c.debug("Client stopped, clearing ephemeral documents");
            // tombstone rather than clear: the refresh below makes VSCode ask
            // for each document again, and there is no longer a server to ask
            for (let key of [...docs.keys()]) {
                docs.set(key, undefined);
                ephemeralEvent.fire(Uri.parse(key));
            }
        }
    })
    cx.subscriptions.push(
        vs.workspace.registerTextDocumentContentProvider(SERVER_NAME, {
            onDidChange: ephemeralEvent.event,
            provideTextDocumentContent: provideEphemeralDocument,
        })
    )
}

/**
 * Handler for {@link COPY_COMMAND}.
 */
async function doCopy(toCopy: unknown): Promise<void> {
    if (typeof toCopy !== "string") {
        client?.error(`${COPY_COMMAND} expected a string argument, got ${typeof toCopy}`);
        void vsWindow.showErrorMessage("Vencord Companion: nothing to copy");
        return;
    }
    await env.clipboard.writeText(toCopy);
    client?.debug(`Copied "${toCopy}" to the clipboard`);
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

function trustHover(hover: Hover): void {
    for (const content of hover.contents) {
        if (content instanceof MarkdownString) {
            content.isTrusted = { enabledCommands: [COPY_COMMAND] };
            content.supportThemeIcons = true;
        }
    }
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
    interface IRange { 
        start: IPosition;
        end: IPosition;
    }
    interface IPosition {
        line: number;
        character: number;
    }
    function convertRange(range: IRange): vs.Range { 
        return new vs.Range(range.start.line, range.start.character, range.end.line, range.end.character);
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
        middleware: {
			// hovers from the server contain `command:` links and `$(icon)`
			// codicons, both of which are inert unless the markdown opts in
			async provideHover(document, position, token, next) {
				const hover = await next(document, position, token);
				if (hover) {
					trustHover(hover);
				}
				return hover;
            },
            window: {
                async showDocument(params, next) { 
                    if (params.selection && !params.takeFocus && !params.external) {
                        let parsedUri = Uri.parse(params.uri);
                        let uriString = parsedUri.toString();
                        let range = convertRange(params.selection);
                        let activeEditor = vsWindow.activeTextEditor;
                        let visibleEditor = activeEditor?.document.uri.toString() === uriString
                            ? activeEditor
                            : vsWindow.visibleTextEditors.find((editor) => {
                                return editor.document.uri.toString() === uriString;
                            });
                        if (visibleEditor) {
                            visibleEditor.selection = new vs.Selection(range.start, range.end);
                            visibleEditor.revealRange(range, vs.TextEditorRevealType.InCenter);
                            return {
                                success: true
                            };
                        }
                        let openTab = vsWindow.tabGroups.all
                            .flatMap((group) => group.tabs)
                            .find((tab) => {
                                return tab.input instanceof vs.TabInputText
                                    && tab.input.uri.toString() === uriString;
                            });
                        if (openTab) {
                            try {
                                await vsWindow.showTextDocument(parsedUri, {
                                    viewColumn: openTab.group.viewColumn,
                                    preserveFocus: true,
                                    selection: range
                                });
                                return {
                                    success: true
                                };
                            } catch {
                                // fall through to the default handler
                            }
                        }
                    }
                    let tokSource = new vs.CancellationTokenSource();
                    using _ = defer(() => tokSource.dispose());
                    // HandlerSignature returns HandlerResult, which permits a
                    // ResponseError; the middleware type does not, so surface it
                    // as a rejection and let the client turn it back into one
                    let res = await next(params, tokSource.token);
                    if (res instanceof ResponseError) {
                        throw res;
                    }
                    return res;
                }
            }
		}
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

function defer(fn: () => void): Disposable { 
    return {
        [Symbol.dispose]() {
            fn();
        }
    };
}

function error(msg?: string): never { 
    throw new Error(msg);
}

/**
 * An assertion with an expression that is always falsy will always fail.
 * 
 * NOTE: NaN is falsy, but not included because there is no literal type for it
 * 
 * @throws an {@link AssertionError} always
 * 
 * @see {@link unreachable} and {@link error} for better uses if you are passing a literal
 * @see {@link https://developer.mozilla.org/en-US/docs/Glossary/Falsy|MDN - Falsy}
 * @see {@link https://developer.mozilla.org/en-US/docs/Web/API/HTMLAllCollection|MDN - HTMLAllCollection}
 */
export function assert(cond: null | undefined | false | 0 | 0n | "", msg?: string): never;
/**
 * Assert {@link cond} is truthy
 * 
 * @throws an {@link AssertionError} if {@link cond} is falsy
 */
export function assert(cond: unknown, msg?: string): asserts cond;
export function assert(cond: unknown, msg?: string): asserts cond {
    if (!cond) {
        const err = new AssertionError(msg);

        AssertionError.captureStackTrace(err, assert);
        throw err;
    }
}

export class AssertionError extends Error {
    override name = "AssertionError";

    constructor(msg?: string) {
        super(msg);
    }
}