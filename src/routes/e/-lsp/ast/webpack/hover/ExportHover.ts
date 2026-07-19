import { ModuleViewerStore, parseModuleURI } from "@/routes/e/-data";
import { copy } from "@/utils/clipboard";
import { GITHUB_REPO_CREATE_ISSUE_URL } from "@/utils/constants";
import { tryMapIntlKey } from "@/utils/discordI18n";
import { error } from "@/utils/error";
import { type Monaco, monaco } from "@/utils/monaco";

import { isWebpackModule } from "../../../util";

interface CopyHoverDataArgs {
    hashedKey: string;
    maybeUnHashedKey: string | null;
}

export class WebpackExportHover implements Monaco.languages.HoverProvider {
    static readonly #COMMAND_NAME = "webpackI18nHover.copy";

    private constructor() {
    }

    async provideHover(
        model: Monaco.editor.ITextModel,
        position: Monaco.Position,
        _token: Monaco.CancellationToken,
        _context?: Monaco.languages.HoverContext,
    ): Promise<Monaco.languages.Hover | null | undefined> {
        try {
            const { buildHash: currentBuildHash, _buildService } = ModuleViewerStore.getState();
            const { buildHash, moduleId } = parseModuleURI(model.uri) ?? {};
            const text = model.getValue();

            if (!isWebpackModule(text)) {
                return;
            }

            if (buildHash !== currentBuildHash) {
                error("Build hash mismatch");
            }

            const { range, content, i18nKey } = await _buildService.generateHover(moduleId!, position) ?? {};

            if (i18nKey) {
                const maybeUnHashedKey = tryMapIntlKey(i18nKey);

                return {
                    range: range!,
                    contents: [
                        {
                            value: maybeUnHashedKey
                              ?? `No mapping found. If you find one, please [open an issue](${GITHUB_REPO_CREATE_ISSUE_URL}) so it can be added!`,
                        },
                        WebpackExportHover.#makeCopyString({
                            hashedKey: i18nKey,
                            maybeUnHashedKey,
                        }),
                    ],
                };
            }

            // also catches empty string for hoverText
            if (!content) {
                return;
            }

            return {
                range: range!,
                contents: [
                    {
                        value: content,
                        isTrusted: true,
                        supportThemeIcons: true,
                    },
                ],
            };
        } catch (e) {
            console.error(e);
        }
    }

    static #makeCopyString(props: CopyHoverDataArgs): Monaco.IMarkdownString {
        const uri = monaco.Uri.parse(`command:${WebpackExportHover.#COMMAND_NAME}?${encodeURIComponent(JSON.stringify([props]))}`);

        return {
            value: `$(copy) [Copy As Find](${uri.toString()})`,
            supportThemeIcons: true,
            isTrusted: {
                enabledCommands: [WebpackExportHover.#COMMAND_NAME],
            },
        };
    }

    static async #handleCopyHoverData(_serviceAccessor: unknown, { hashedKey, maybeUnHashedKey }: CopyHoverDataArgs) {
        const toCopy = maybeUnHashedKey
            ? `#{intl::${maybeUnHashedKey}}`
            : `#{intl::${hashedKey}::raw}`;

        await copy(toCopy);
    }

    public static register() {
        monaco.languages.registerHoverProvider({ language: "javascript" }, new WebpackExportHover());
        monaco.editor.registerCommand(WebpackExportHover.#COMMAND_NAME, WebpackExportHover.#handleCopyHoverData);
    }
}
