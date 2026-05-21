import { assert, error } from "@/utils/error";
import { dedent } from "@/utils/string";
import { makeLegalIdentifier } from "@rollup/pluginutils";

import metadata, { type EditorFeature, type EditorLanguage, type IFeatureDefinition, type IWorkerDefinition, type NegatedEditorFeature } from "monaco-editor/esm/metadata.js";
import { mkdir, writeFile } from "node:fs/promises";
import { posix, resolve } from "node:path";


interface MonacoEditorOptions {
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

const OPTIONS = Object.freeze({
    languages: ["typescript", "javascript"],
} satisfies MonacoEditorOptions as MonacoEditorOptions);


function fixMetadata() {
    const cssWorker = metadata.languages.find((lang) => lang.label === "css")?.worker;
    const htmlWorker = metadata.languages.find((lang) => lang.label === "html")?.worker;
    const tsWorker = metadata.languages.find((lang) => lang.label === "typescript")?.worker;

    assert(htmlWorker, "html worker not found");
    assert(cssWorker, "css worker not found");
    assert(tsWorker, "typescript worker not found");

    for (const lang of metadata.languages) {
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
}

fixMetadata();

interface MonacoWorker extends IWorkerDefinition {
    name: string;
}

const resolvedLanguages: IFeatureDefinition[] = [];
const resolvedFeatures: IFeatureDefinition[] = [];

const resolvedWorkers: MonacoWorker[] = [
    {
        name: "editorWorkerService",
        id: "vs/editor/editor",
        entry: "vs/editor/editor.worker",
    },
];

const { languages, features } = OPTIONS;
const rootDir = resolve(import.meta.dirname, "..", "..");
const monacoWorkerPrefix = resolve(rootDir, "node_modules", "monaco-editor", "esm");
const outDir = resolve(rootDir, "src", "utils", "monaco", "generated");
const outPath = resolve(outDir, "entry.ts");

await mkdir(outDir, { recursive: true });

// resolve languages
if (languages) {
    if (languages.length) {
        const toAdd = metadata.languages.filter((lang) => languages.includes(lang.label as EditorLanguage));

        resolvedLanguages.push(...toAdd);
    } else {
        console.debug("Bundling monaco-editor with no languages enabled");
        console.debug("To select all languages, pass undefined instead of an empty array");
    }
} else {
    resolvedLanguages.push(...metadata.languages);
}

// resolve features
if (features) {
    const { add = [], remove = [] } = Object.groupBy(features, (f) => (f.startsWith("!") ? "remove" : "add"));

    if (add.length && remove.length) {
        error("Cannot mix negated and non-negated features");
    } else if (add.length) {
        const toAdd = metadata.features.filter((feature) => add.includes(feature.label as EditorFeature));

        resolvedFeatures.push(...toAdd);
    } else if (remove.length) {
        const toAdd = metadata.features.filter((feature) => !remove.includes(`!${feature.label as EditorFeature}`));

        console.log(toAdd);
        resolvedFeatures.push(...toAdd);
    } else {
        console.debug("Bundling monaco-editor with no features enabled");
        console.debug("To select all features, pass undefined instead of an empty array");
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

interface WorkerInfo {
    names: Set<string>;
    refId: string;
}

const emitWorkers = new Map<string, WorkerInfo>();

for (const { id, entry, name } of resolvedWorkers) {
    if (emitWorkers.has(id)) {
        emitWorkers.get(id)!.names.add(name);
        continue;
    }

    const fullWorkerPath = resolve(monacoWorkerPrefix, entry);
    const workerEntryId = posix.relative(outDir, fullWorkerPath);
    const refId = `${workerEntryId}`;
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

const generatedContent = dedent/*js*/`
    /* eslint-disable */
    // This file is generated by scripts/codegen/monacoEditor.ts. Do not edit.
    ${emitWorkers.entries().map(([id, { refId }]) => {
        const ident = makeLegalIdentifier(id);

        return dedent/*js*/`
            const ${ident} = (await ((import.meta.env.DEV && !import.meta.env.SSR) 
                ? import("${refId}?worker&url")
                : import("omt:${refId}")
            )).default
        `;
    })
        .toArray()
        .join("\n")}
    globalThis.MonacoEnvironment = {
        getWorker(_name, label) {
            switch (label) {
                ${emitWorkers
                    .entries()
                    .flatMap(([id, { names }]) => [
                        ...names
                            .values()
                            .map((name) => /*js*/`case "${name}":`)
                            .toArray(),
                        /*js*/`return new Worker(${makeLegalIdentifier(id)}, {type: "module", name: "${id}" });`,
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

await writeFile(outPath, generatedContent);
