import _metadata, { type EditorFeature, type EditorLanguage, type IFeatureDefinition, type IWorkerDefinition, type NegatedEditorFeature } from "monaco-editor/esm/metadata.js";
import type { Plugin } from "vite";

export interface MonacoEditorOptions {
    /**
     * Features to include
     *
     * pass an empty array to include nothing
     * 
     * pass undefined to include everything
     */
    features?: EditorFeature[] | NegatedEditorFeature[];
    /**
     * Languages to include
     *
     * pass an empty array to include nothing
     *
     * pass undefined to include everything
     */
    languages?: EditorLanguage[];
}


function fixMetadata(orig: typeof _metadata): typeof _metadata {
    const ret = structuredClone(orig);
    const cssWorker = ret.languages.find((lang) => lang.label === "css")?.worker;
    const htmlWorker = ret.languages.find((lang) => lang.label === "html")?.worker;
    const tsWorker = ret.languages.find((lang) => lang.label === "typescript")?.worker;

    assert(htmlWorker, "html worker not found");
    assert(cssWorker, "css worker not found");
    assert(tsWorker, "typescript worker not found");

    for (const lang of ret.languages) {
        if (lang.label === "scss" || lang.label === "less") {
            assert(!lang.worker, `lang ${lang.label} already has a worker`);
            lang.worker = cssWorker;
        } else if (lang.label === "handlebars" || lang.label === "razor") {
            assert(!lang.worker, `lang ${lang.label} already has a worker`);
            lang.worker = htmlWorker;
        } else if (lang.label === "javascript") {
            assert(!lang.worker, `lang ${lang.label} already has a worker`);
            lang.worker = tsWorker;
        }
    }
    return ret;
}

interface MonacoWorker extends IWorkerDefinition {
    name: string;
}

const MONACO_VIRTUAL_MODULE_ID = "\0bundle-monaco-editor";

export function monacoEditor({ features, languages }: MonacoEditorOptions = {}): Plugin {
    let resolvedLanguages: IFeatureDefinition[];
    let resolvedFeatures: IFeatureDefinition[];
    let resolvedWorkers: MonacoWorker[];
    let isServe = false;

    return {
        name: "bundle-monaco-editor",
        buildStart() {
            resolvedLanguages = [];
            resolvedFeatures = [];
            resolvedWorkers = [
                {
                    name: "editorWorkerService",
                    id: "vs/editor/editor",
                    entry: "vs/editor/editor.worker",
                },
            ];

            const metadata = fixMetadata(_metadata);

            // languages
            if (languages) {
                if (languages.length) {
                    const toAdd = metadata.languages.filter((lang) => languages.includes(lang.label as EditorLanguage));

                    resolvedLanguages.push(...toAdd);
                } else {
                    this.debug("Bundling monaco-editor with no languages enabled");
                    this.debug("To select all languages, pass undefined instead of an empty array");
                }
            } else {
                resolvedLanguages.push(...metadata.languages);
            }

            // Resolve features
            if (features) {
                const { add = [], remove = [] } = Object.groupBy(features, (f) => (f.startsWith("!") ? "remove" : "add"));

                if (add.length && remove.length) {
                    this.error("Cannot mix negated and non-negated features");
                } else if (add.length) {
                    const toAdd = metadata.features.filter((feature) => add.includes(feature.label as EditorFeature));

                    resolvedFeatures.push(...toAdd);
                } else if (remove.length) {
                    const toAdd = metadata.features.filter((feature) => !remove.includes(`!${feature.label as EditorFeature}`));

                    console.log(toAdd);
                    resolvedFeatures.push(...toAdd);
                } else {
                    this.debug("Bundling monaco-editor with no features enabled");
                    this.debug("To select all features, pass undefined instead of an empty array");
                }
            } else {
                resolvedFeatures.push(...metadata.features);
            }

            for (const { label, worker } of resolvedFeatures) {
                if (worker) {
                    resolvedWorkers.push({
                        ...worker,
                        name: label,
                    });
                }
            }

            for (const { label, worker } of resolvedLanguages) {
                if (worker) {
                    resolvedWorkers.push({
                        ...worker,
                        name: label,
                    });
                }
            }
        },
        configureServer() {
            isServe = true;
        },
        resolveId: {
            order: "pre",
            filter: {
                id: /^monaco-editor$/,
            },
            handler(source) {
                if (source === "monaco-editor") {
                    return MONACO_VIRTUAL_MODULE_ID;
                }
                this.error("unreachable");
            },
        },
        load: {
            order: "pre",
            filter: {
                id: /bundle-monaco-editor$/,
            },
            handler(id) {
                if (id !== MONACO_VIRTUAL_MODULE_ID) {
                    return;
                }

                type Id = string;

                interface WorkerInfo {
                    names: Set<string>;
                    refId: string;
                }

                const emitWorkers = new Map<Id, WorkerInfo>();

                for (const { id, entry, name } of resolvedWorkers) {
                    if (emitWorkers.has(id)) {
                        emitWorkers.get(id)!.names.add(name);
                        continue;
                    }

                    const workerEntryId = `monaco-editor/esm/${entry}.js`;

                    const refId = isServe
                        ? `new URL("${workerEntryId}", import.meta.url)`
                        : `import.meta.ROLLUP_FILE_URL_${this.emitFile({
                            type: "chunk",
                            id: workerEntryId,
                        })}`;

                    const names = new Set([name]);

                    emitWorkers.set(id, {
                        names,
                        refId,
                    });
                }


                function makeImports(features: IFeatureDefinition[]): string {
                    return features
                        .filter(({ entry }) => !!entry)
                        .map(({ entry: entries, ...rest }) => ({
                            entries: Array.isArray(entries) ? entries : [entries ?? assert(false)],
                            ...rest,
                        }))
                        .flatMap(({ label, entries }) => entries.map((entry) => {
                            const path = entry.endsWith(".css") ? entry : `${entry}.js`;

                            return dedent/*js*/`
                                // ${label}
                                import "monaco-editor/esm/${path}";
                            `;
                        }))
                        .join("\n");
                }

                const ret = dedent/*js*/`
                    globalThis.MonacoEnvironment = {
                        getWorker(name, label) {
                            switch (label) {
                                ${emitWorkers
                                    .entries()
                                    .flatMap(([id, { names, refId }]) => [
                                        ...names
                                            .values()
                                            .map((name) => /*js*/`case "${name}":`)
                                            .toArray(),
                                        /*js*/`return new Worker(${refId}, {type: "module", name: "${id}" });`,
                                    ])
                                    .toArray()
                                    .join("\n")}
                            }
                            throw new Error(${"`"}No worker found for label: ${"$"}{label}${"`"});
                        }
                    };
                    // Features
                    ${makeImports(resolvedFeatures)}
                    // Monaco Core
                    import * as monaco from "monaco-editor/esm/vs/editor/editor.api.js";
                    export * from "monaco-editor/esm/vs/editor/editor.api.js";
                    export default monaco;
                    // Languages
                    ${makeImports(resolvedLanguages)}
                `;

                return ret;
            },
        },
    };
}

function assert(cond: null | undefined | false | 0 | -0 | 0n | "" | HTMLAllCollection, msg?: string): never;
function assert(cond: unknown, msg?: string): asserts cond;
function assert(cond: unknown, msg?: string): asserts cond {
    if (!cond) {
        throw new Error(msg ?? "Assertion failed");
    }
}

function dedent(literals: string): string;
function dedent(strings: TemplateStringsArray, ...values: unknown[]): string;
function dedent(
    strings: TemplateStringsArray | string,
    ...values: unknown[]
) {
    /*!
     * https://github.com/dmnd/dedent
     * @license MIT
     */
    const raw = typeof strings === "string" ? [strings] : strings.raw;
    // first, perform interpolation
    let result = "";

    for (let i = 0; i < raw.length; i++) {
        result += raw[i];

        if (i < values.length) {
            const value = alignValue(values[i], result);


            result += value;
        }
    }

    // now strip indentation
    const lines = result.split("\n");
    let mindent: null | number = null;

    for (const l of lines) {
        const m = l.match(/^(\s+)\S+/);

        if (m) {
            const indent = m[1].length;

            if (!mindent) {
                // this is the first indented line
                mindent = indent;
            } else {
                mindent = Math.min(mindent, indent);
            }
        }
    }

    if (mindent !== null) {
        result = lines
        // https://github.com/typescript-eslint/typescript-eslint/issues/7140
            .map((l) => (l[0] === " " || l[0] === "\t" ? l.slice(mindent) : l))
            .join("\n");
    }

    // dedent eats leading and trailing whitespace too
    result = result.trim();

    return result;
    /**
     * Adjusts the indentation of a multi-line interpolated value to match the current line.
     */
    function alignValue(value: unknown, precedingText: string): string | unknown {
        if (typeof value !== "string" || !value.includes("\n")) {
            return value;
        }

        const currentLine = precedingText.slice(precedingText.lastIndexOf("\n") + 1);
        const indentMatch = currentLine.match(/^(\s+)/);

        if (indentMatch) {
            const [indent] = indentMatch;

            return value.replace(/\n/g, `\n${indent}`);
        }

        return value;
    }
}
