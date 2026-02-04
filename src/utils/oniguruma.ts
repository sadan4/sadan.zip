import type { IOnigLib } from "@/components/CodeEditor/Monaco/vscode-textmate/main";
import { makeLazy } from "@/utils/lazy";

import type { RegexEngine } from "shiki";
import * as _vscodeOniguruma from "vscode-oniguruma";
import onigurumaWasmURL from "vscode-oniguruma/release/onig.wasm?url";

const vscodeOniguruma = import.meta.env.SSR ? (_vscodeOniguruma as any).default as never : _vscodeOniguruma;

export const loadOnigasmPromise = makeLazy(async () => {
    if (import.meta.env.SSR) {
        // const { fileURLToPath } = await import("node:url");
        // const { readFile } = await import("node:fs/promises");
        // const wasmPath = fileURLToPath(import.meta.resolve("vscode-oniguruma/release/onig.wasm"));

        // return vscodeOniguruma.loadWASM(await readFile(wasmPath));
        // @ts-expect-error cloudflare/vite-plugin handles this import
        const { default: onigWasmModule } = await import("vscode-oniguruma/release/onig.wasm") as { default: WebAssembly.Module; };

        return vscodeOniguruma.loadWASM({
            async instantiator(importObject) {
                return {
                    instance: await WebAssembly.instantiate(onigWasmModule, importObject),
                    module: onigWasmModule,
                };
            },
        });
    }

    const res = await fetch(onigurumaWasmURL);

    return vscodeOniguruma.loadWASM(res);
});

export const createOnigurumaEngine = makeLazy(async () => {
    return loadOnigasmPromise().then((): IOnigLib & RegexEngine => {
        return {
            createOnigScanner(sources) {
                return new vscodeOniguruma.OnigScanner(sources);
            },
            createOnigString(str) {
                return new vscodeOniguruma.OnigString(str);
            },
            createScanner(patterns) {
                return new vscodeOniguruma.OnigScanner(patterns.map((p) => (typeof p === "string" ? p : p.source)));
            },
            createString(s) {
                return new vscodeOniguruma.OnigString(s);
            },
        };
    });
});
