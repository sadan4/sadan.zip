import { MonacoCodeEditor } from "@/components/CodeEditor/Monaco";
import { error, unreachable } from "@/utils/error";
import { debounce } from "@/utils/functional";
import { inRangeExclusive } from "@/utils/math";
import { type Monaco, monaco } from "@/utils/monaco";
import { Language } from "@/utils/textmate";
import { useSuspenseQuery } from "@tanstack/react-query";

import { ModuleViewerStore, useModuleViewerStore } from "../-data";

import * as css from "css-tree";
import { useEffect, useMemo, useState } from "react";

const PROVIDER_KEY = "discord-css-class-editor";
const MARKER_NS = "discord-css-class-editor-markers";
const URI = monaco.Uri.parse(`file:///${PROVIDER_KEY}.css`);

// TODO: move to worker
function reportUnknownClasses(code: string, classes: Set<string>): Monaco.editor.IMarkerData[] {
    const ret: Monaco.editor.IMarkerData[] = [];

    function report({ name, loc = unreachable() }: css.ClassSelector) {
        ret.push({
            startLineNumber: loc.start.line,
            startColumn: loc.start.column,
            endLineNumber: loc.end.line,
            endColumn: loc.end.column,
            severity: monaco.MarkerSeverity.Warning,
            message: `class \`.${name}\` is not used by discord at all`,
        });
    }

    const ast = css.parse(code, {
        positions: true,
    });

    css.walk(ast, {
        enter(node) {
            if (!classes.has(node.name)) {
                report(node);
            }
        },
        visit: "ClassSelector",
    });
    return ret;
}

function nodeFromPos(ast: css.CssNode, offset: number): css.CssNode {
    let cur = ast;

    css.walk(ast, {
        enter(node: css.CssNode) {
            const { start: { offset: start }, end: { offset: end } } = node.loc!;

            if (inRangeExclusive(start, end, offset)) {
                cur = node;
            }
        },
    });
    return cur;
}

// The css var completion is lifted from tailwind
// https://github.com/tailwindlabs/tailwindcss-intellisense/blob/5067ff7ec0b4b8bb5c3325a69c1f3410eb5b350a/packages/tailwindcss-language-service/src/completionProvider.ts
const VAR_REGEX = /[\s:;/*(){}](?<helper>var)\(\s*['"]?(?<path>[^)'"]*)$/;

export function CssEditor() {
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const [handle, setHandle] = useState<null | MonacoCodeEditor.Handle>(null);

    const { data: cssModules } = useSuspenseQuery({
        queryKey: ["get css modules", buildHash],
        queryFn() {
            return buildService.getCssModules();
        },
    });

    const { data: cssVars } = useSuspenseQuery({
        queryKey: ["get css vars", buildHash],
        queryFn() {
            return buildService.getCssVars();
        },
    });

    const namesMap = useMemo(() => {
        return new Map(cssModules.flatMap((x) => x.class_names.map((x) => [x.value.split(" ")[0], x])));
    }, [cssModules]);

    // oxlint-disable-next-line react-x/no-unnecessary-use-memo -- easier to reason about
    const namesList = useMemo(() => new Set(namesMap.keys()), [namesMap]);


    // FIXME: for now, it's easier to reason about
    // oxlint-disable-next-line react-x/no-unnecessary-use-memo
    const cssInfo: Monaco.languages.css.CSSDataV1 = useMemo(() => {
        const names = new Map<string, Monaco.languages.css.IPseudoClassData>();

        for (const module of cssModules) {
            for (const clazz of module.class_names) {
                const [name] = clazz.value.split(" ");

                if (names.has(name)) {
                    continue;
                }

                const data: Monaco.languages.css.IPseudoClassData = {
                    name: `.${name}`,
                    description: `defined in ${clazz.module}, exported as ${clazz.exported_as}`,
                };

                names.set(name, data);
            }
        }

        const pseudoElements = [...names.values()];

        return {
            version: 1.1,
            pseudoElements,
        };
    }, [cssModules]);

    useEffect(() => {
        const opts = monaco.languages.css.cssDefaults.options;

        // oxlint-disable-next-line logical-assignment-operators -- react compiler doesn't like it
        if (opts.data!.dataProviders == null) {
            opts.data!.dataProviders = {};
        }

        // oxlint-disable-next-line prefer-destructuring -- typescript bug?
        const dataProviders = opts.data!.dataProviders;

        if (PROVIDER_KEY in dataProviders) {
            error("our data provider is already set");
        }
        dataProviders[PROVIDER_KEY] = cssInfo;
        monaco.languages.css.cssDefaults.setOptions(opts);
        return () => void delete dataProviders[PROVIDER_KEY];
    }, [cssInfo]);

    useEffect(() => {
        const editor = handle?.editor;

        if (!editor) {
            return;
        }

        const model = editor.getModel();

        if (!model) {
            return;
        }

        function clearMarkers() {
            monaco.editor.setModelMarkers(model!, MARKER_NS, []);
        }

        const updateMarkers = debounce(function doUpdate() {
            const source = model.getValue();

            try {
                const markers = reportUnknownClasses(source, namesList);

                monaco.editor.setModelMarkers(model, MARKER_NS, markers);
            } catch {
                clearMarkers();
            }
        }, 1000);

        const removeListener = model.onDidChangeContent(updateMarkers);

        const removeDefProvider = monaco.languages.registerDefinitionProvider({ language: "css" }, {
            async provideDefinition(
                givenModel: Monaco.editor.ITextModel,
                position: Monaco.Position,
                // _token: Monaco.CancellationToken,
            ): Promise<Monaco.languages.Definition | Monaco.languages.LocationLink[] | undefined | null> {
                if (givenModel !== model) {
                    return;
                }

                const text = model.getValue();

                const ast = css.parse(text, {
                    positions: true,
                });

                const node = nodeFromPos(ast, model.getOffsetAt(position));

                if (node.type !== "ClassSelector") {
                    return;
                }

                const obj = namesMap.get(node.name);

                if (!obj) {
                    return;
                }

                const sourceModel = await ModuleViewerStore.getState().getModuleModel(obj.module);
                const start = sourceModel.getPositionAt(obj.span.start);
                const end = sourceModel.getPositionAt(obj.span.end);

                return {
                    uri: sourceModel.uri,
                    range: {
                        startLineNumber: start.lineNumber,
                        startColumn: start.column,
                        endLineNumber: end.lineNumber,
                        endColumn: end.column,
                    },
                } satisfies Monaco.languages.Definition;
            },
        } satisfies Monaco.languages.DefinitionProvider);

        const removeCompletionProvider = monaco.languages.registerCompletionItemProvider(
            { language: "css" },
            {
                triggerCharacters: ["(", "-"],

                provideCompletionItems(model, position) {
                    // eslint-disable-next-line prefer-template
                    const prefix = " "
                      + model.getValueInRange({
                          startLineNumber: position.lineNumber,
                          startColumn: 1,
                          endLineNumber: position.lineNumber,
                          endColumn: position.column,
                      });

                    const match = prefix.match(VAR_REGEX);

                    if (!match?.groups)
                        return { suggestions: [] };

                    const partial = match.groups.path;

                    const range = new monaco.Range(
                        position.lineNumber,
                        position.column - partial.length,
                        position.lineNumber,
                        position.column,
                    );

                    const pad = String(cssVars.length).length;

                    const suggestions = cssVars.map((v, i): Monaco.languages.CompletionItem => {
                        return {
                            label: v,
                            kind: monaco.languages.CompletionItemKind.Variable,
                            detail: `detail for ${v}`,
                            insertText: v,
                            range,
                            // sort in declaration order
                            // bump ours to the top over the thousands of css classes
                            sortText: `_a_${String(i).padStart(pad, "0")}`,
                        };
                    });

                    return { suggestions };
                },
            } satisfies Monaco.languages.CompletionItemProvider,
        );

        return () => {
            removeListener.dispose();
            removeDefProvider.dispose();
            removeCompletionProvider.dispose();
            clearMarkers();
        };
    }, [handle, namesList, namesMap, cssVars]);

    return (
        <MonacoCodeEditor
            language={Language.CSS}
            uri={URI}
            ref={setHandle}
        />
    );
}
