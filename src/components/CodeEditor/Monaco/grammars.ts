import { makeLazy } from "@/utils/lazy";
import { createOnigurumaEngine } from "@/utils/oniguruma";
import { hasGrammar, lazyLoadGrammar } from "@/utils/textmate";

import * as monaco from "monaco-editor";
import { INITIAL, type IRawGrammar, Registry, type StateStack } from "vscode-textmate";

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

class TokenizerState implements monaco.languages.IState {
    constructor(private _ruleStack: StateStack) { }

    public get ruleStack(): StateStack {
        return this._ruleStack;
    }

    public clone(): TokenizerState {
        return new TokenizerState(this._ruleStack);
    }

    public equals(other: monaco.languages.IState): boolean {
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
export function wireTmGrammars(registry: Registry, languages: Map<string, string>, editor?: monaco.editor.ICodeEditor) {
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

            monaco.languages.setTokensProvider(languageId, {
                getInitialState: () => new TokenizerState(INITIAL),
                tokenizeEncoded(line, state: TokenizerState) {
                    const { ruleStack, tokens } = grammar.tokenizeLine2(line, state.ruleStack);

                    const testingTokens = tokens.map((token, index) => {
                        if (index % 2 === 1) {
                            return 0b0000_0000_0000_0000_1100_0000_0000_0000 << 1;
                        }
                        return token;
                    });

                    window.monaco = monaco;

                    return {
                        endState: new TokenizerState(ruleStack),
                        tokens: testingTokens,
                    };
                },
                // tokenize(line: string, state: TokenizerState) {
                //     const { ruleStack, tokens } = grammar.tokenizeLine(line, state.ruleStack);

                //     return {
                //         endState: new TokenizerState(ruleStack),
                //         tokens: tokens.map((token) => ({
                //             ...token,
                //             // TODO: At the moment, monaco-editor doesn't seem to accept array of scopes
                //             // scopes: editor ? TMToMonacoToken(editor, token.scopes) : token.scopes.at(-1)!,
                //             scopes: token.scopes,
                //         })),
                //     };
                // },
            });
        }));
}

// as described in issue: https://github.com/NeekSandhu/monaco-textmate/issues/5
function TMToMonacoToken(editor: monaco.editor.ICodeEditor, scopes: string[]) {
    let scopeName = "";

    // get the scope name. Example: cpp , java, haskell
    for (let i = scopes[0].length - 1; i >= 0; i -= 1) {
        const char = scopes[0][i];

        if (char === ".") {
            break;
        }
        scopeName = char + scopeName;
    }

    // iterate through all scopes from last to first
    for (let i = scopes.length - 1; i >= 0; i -= 1) {
        const scope = scopes[i];

        /**
         * Try all possible tokens from high specific token to low specific token
         *
         * Example:
         * 0 meta.function.definition.parameters.cpp
         * 1 meta.function.definition.parameters
         *
         * 2 meta.function.definition.cpp
         * 3 meta.function.definition
         *
         * 4 meta.function.cpp
         * 5 meta.function
         *
         * 6 meta.cpp
         * 7 meta
         */
        for (let i = scope.length - 1; i >= 0; i -= 1) {
            const char = scope[i];

            if (char === ".") {
                const token = scope.slice(0, i);

                if ((editor as any)._themeService._theme._tokenTheme._match(`${token}.${scopeName}`)._foreground
                  > 1) {
                    return `${token}.${scopeName}`;
                }
                if ((editor as any)._themeService._theme._tokenTheme._match(token)._foreground > 1) {
                    return token;
                }
            }
        }
    }

    return "";
}
