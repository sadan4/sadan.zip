import { ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import { copy } from "@/utils/clipboard";
import { GITHUB_REPO_CREATE_ISSUE_URL } from "@/utils/constants";
import { tryMapIntlKey } from "@/utils/discordI18n";
import { error } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";
import { AstParser } from "@vencord-companion/ast-parser";

import { isWebpackModule, toMonacoRange, toParserPosition } from "../../../util";

import { isElementAccessExpression, isIdentifier, isPropertyAccessExpression, isStringLiteralLike } from "typescript";

interface CopyHoverDataArgs {
    hashedKey: string;
    maybeUnHashedKey: string | null;
}

// TODO: Move to worker?
export class WebpackI18nHover implements Monaco.languages.HoverProvider {
    static #COMMAND_NAME = "webpackI18nHover.copy";

    private constructor() {

    }

    provideHover(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
        _token: Monaco.CancellationToken,
        _context?: Monaco.languages.HoverContext<Monaco.languages.Hover> | undefined,
    ): Monaco.languages.Hover | undefined {
        try {
            const { buildHash } = ModuleViewerStore.getState();
            const parsedUri = parseModuleURI(model.uri);
            const text = model.getValue();

            if (parsedUri?.buildHash !== buildHash) {
                error("Build hash mismatch");
            }
            if (!isWebpackModule(text)) {
                return;
            }

            const parser = new AstParser(text);
            const pos = toParserPosition(position);
            const node = parser.getTokenAtPosition(pos);

            if (!node) {
                return;
            }

            // intl calls are either \i.t.<key> or \i.t["key"]
            // node will be key, node.parent will be a property access or element access expression
            const { parent } = node;

            if (!isPropertyAccessExpression(parent) && !isElementAccessExpression(parent)) {
                return;
            }

            let hashedKey: string | undefined;

            if (isIdentifier(node)) {
                hashedKey = node.getText();
            } else if (isStringLiteralLike(node)) {
                hashedKey = node.text;
            } else {
                console.warn("[WebpackI18nHover] Unrecognized node type for i18n key:", node.kind);
                return;
            }

            if (!hashedKey) {
                console.warn("[WebpackI18nHover] Missing hashed i18n key.");
                return;
            }

            if (hashedKey.length !== 6) {
                console.warn(`[WebpackI18nHover] Expected hashed i18n key to be 6 characters long, got ${hashedKey.length} for key: ${hashedKey}`);
                return;
            }
            // TODO: find the intl chunk and try to get the string

            const maybeUnHashedKey = tryMapIntlKey(hashedKey);

            return {
                range: toMonacoRange(parser.makeRangeFromAstNode(node)),
                contents: [
                    {
                        value: maybeUnHashedKey
                          ?? `No mapping found. If you find one, please [open an issue](${GITHUB_REPO_CREATE_ISSUE_URL}) so it can be added!`,
                    },
                    WebpackI18nHover.#makeCopyString({
                        hashedKey,
                        maybeUnHashedKey,
                    }),
                ],
            };
        } catch (e) {
            console.error("[WebpackI18nHover]:", e);
        }
    }

    static #makeCopyString(props: CopyHoverDataArgs): Monaco.IMarkdownString {
        const uri = WebpackI18nHover.#createCommandUri(props).toString();

        return {
            value: `$(copy) [Copy As Find](${uri})`,
            supportThemeIcons: true,
            isTrusted: {
                enabledCommands: [WebpackI18nHover.#COMMAND_NAME],
            },
        };
    }

    static async #handleCopyHoverData(_serviceAccessor: unknown, { hashedKey, maybeUnHashedKey }: CopyHoverDataArgs) {
        const toCopy = maybeUnHashedKey
            ? `#{intl::${maybeUnHashedKey}}`
            : `#{intl::${hashedKey}::raw}`;

        await copy(toCopy);
    }

    static #createCommandUri(props: CopyHoverDataArgs): Monaco.Uri {
        return monaco.Uri.parse(`command:${WebpackI18nHover.#COMMAND_NAME}?${encodeURIComponent(JSON.stringify([props]))}`);
    }

    public static register() {
        monaco.languages.registerHoverProvider({ language: "javascript" }, new WebpackI18nHover());
        monaco.editor.registerCommand(WebpackI18nHover.#COMMAND_NAME, WebpackI18nHover.#handleCopyHoverData);
    }
}
