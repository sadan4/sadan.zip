import type { IOnigLib } from "@/components/CodeEditor/Monaco/vscode-textmate/main";
import { makeLazy } from "@/utils/lazy";

import type { RegexEngine } from "shiki";
import * as _vscodeOniguruma from "vscode-oniguruma";
import onigurumaWasmURL from "vscode-oniguruma/release/onig.wasm?url";

const vscodeOniguruma = IS_CLOUDFLARE ? (_vscodeOniguruma as any).default as never : _vscodeOniguruma;

export const loadOnigasmPromise = makeLazy(async () => {
    if (import.meta.env.SSR) {
        if (IS_CLOUDFLARE) {
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
        // eslint-disable-next-line no-else-return -- easier for bundlers to tree-shake
        } else {
            const { readFile } = await import("node:fs/promises");
            const { resolve } = await import("node:path");
            // rspack is buggy and return a relative path instead of a absolute URI
            const resolved = resolve(import.meta.resolve("vscode-oniguruma/release/onig.wasm"));

            return vscodeOniguruma.loadWASM(await readFile(resolved));
        }
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
