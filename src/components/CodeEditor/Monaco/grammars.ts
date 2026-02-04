import { error } from "@/utils/error";
import { makeLazy } from "@/utils/lazy";
import { type Monaco, monaco } from "@/utils/monaco";
import { createOnigurumaEngine } from "@/utils/oniguruma";
import { hasGrammar, lazyLoadGrammar } from "@/utils/textmate";

import { INITIAL, type IRawGrammar, Registry, type StateStack } from "./vscode-textmate/main";
import { ColorMap, ScopeStack, Theme } from "./vscode-textmate/theme";

export const registry = makeLazy(() => {
    return new Registry({
        onigLib: createOnigurumaEngine(),
        async loadGrammar(scopeName) {
            if (hasGrammar(scopeName)) {
                const content = await lazyLoadGrammar(scopeName);

                return content as IRawGrammar;
            }
            console.warn(`No grammar found for scope name: ${scopeName}`);
            return null;
        },
    });
});

interface ThemeSetting {
    scope: string;
    settings: {
        foreground?: string;
        background?: string;
        fontStyle?: string;
    };
}

class TokenizerState implements Monaco.languages.IState {
    constructor(private _ruleStack: StateStack) { }

    public get ruleStack(): StateStack {
        return this._ruleStack;
    }

    public clone(): TokenizerState {
        return new TokenizerState(this._ruleStack);
    }

    public equals(other: Monaco.languages.IState): boolean {
        if (!other
          || !(other instanceof TokenizerState)
          || other !== this
          || other._ruleStack !== this._ruleStack
        ) {
            return false;
        }
        return true;
    }
}

/**
 * Wires up monaco-editor with monaco-textmate
 *
 * @param registry TmGrammar `Registry` this wiring should rely on to provide the grammars
 * @param languages `Map` of language ids (string) to TM names (string)
 */
export function wireTmGrammars(
    registry: Registry,
    languages: Map<string, string>,
    editor: Monaco.editor.ICodeEditor,
    theme: string,
) {
    return Promise.all(Array.from(languages.keys())
        .map(async (languageId) => {
            const lang = languages.get(languageId);

            if (!lang) {
                console.error(`No language found for id: ${languageId}`);
                return;
            }

            const grammar = await registry.loadGrammar(lang);

            if (!grammar) {
                console.debug(`No grammar loaded for language: ${lang}`);
                return;
            }

            const settings = getRulesForTheme(editor, theme);
            const colorMap = (editor as any)._themeService._theme._tokenTheme._colorMap as ColorMap;

            const themeMatcher = Theme.createFromRawTheme({ settings }, new class extends ColorMap {
                override getColorMap(): string[] {
                    return colorMap.getColorMap();
                }

                override getId(color: string | null): number {
                    return colorMap.getId(color);
                }
            }());

            monaco.languages.setTokensProvider(languageId, {
                getInitialState: () => new TokenizerState(INITIAL),
                tokenizeEncoded(line, state: TokenizerState) {
                    const { ruleStack, tokens } = grammar.tokenizeLine(line, state.ruleStack);
                    const ret = new Uint32Array(tokens.length * 2);

                    for (let i = 0; i < tokens.length; i++) {
                        const tok = tokens[i];
                        const pos = i * 2;

                        const {
                            backgroundId,
                            foregroundId,
                            fontStyle,
                        } = themeMatcher.match(ScopeStack.fromArray(tok.scopes)) ?? themeMatcher.getDefaults();

                        ret[pos] = tok.startIndex;

                        const metadata = 0
                          | ((fontStyle === -1 ? 0 : fontStyle & ((1 << 4) - 1)) << 11)
                          | ((foregroundId & ((1 << 9) - 1)) << 15)
                          | ((backgroundId & ((1 << 9) - 1)) << 24);

                        ret[pos + 1] = metadata;
                    }

                    return {
                        endState: new TokenizerState(ruleStack),
                        tokens: ret,
                    };
                },
            });
        }));
}

const themeRulesCache: Map<string, ThemeSetting[]> = new Map();

function getRulesForTheme(editor: Monaco.editor.ICodeEditor, theme: string): ThemeSetting[] {
    if (themeRulesCache.has(theme)) {
        return themeRulesCache.get(theme)!;
    }

    const knownThemes: Map<string, any> = (editor as any)._themeService._knownThemes;
    const themeData = knownThemes.get(theme);

    if (!themeData) {
        error(`could not find theme data for theme: ${theme}`);
    }

    const rules = themeData.themeData.rules as Monaco.editor.ITokenThemeRule[];

    const themeSettings: ThemeSetting[] = rules.map(({ token: scope, ...settings }) => {
        return {
            scope,
            settings,
        };
    });

    themeRulesCache.set(theme, themeSettings);

    return themeSettings;
}
