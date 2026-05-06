import type { IOnigLib } from "@/components/CodeEditor/Monaco/vscode-textmate/main";
import { makeLazy } from "@/utils/lazy";

import type { RegexEngine } from "shiki";
import * as vscodeOniguruma from "vscode-oniguruma";
import onigurumaWasmURL from "vscode-oniguruma/release/onig.wasm?url";

export const loadOnigasmPromise = makeLazy(async () => {
    if (import.meta.env.SSR) {
        notCloudflare: if (IS_CLOUDFLARE) {
            let onigWasmModule: WebAssembly.Module;
            const guh3 = __webpack_require__(import.meta.resolve("vscode-oniguruma/release/onig.wasm")) as string;

            try {
                ({ default: onigWasmModule }
                    = await import(/* webpackIgnore: true */guh3) as { default: WebAssembly.Module; });
            } catch {
                break notCloudflare;
            }

            if (onigWasmModule instanceof WebAssembly.Module) {
                return vscodeOniguruma.loadWASM({
                    async instantiator(importObject) {
                        return {
                            instance: await WebAssembly.instantiate(onigWasmModule, importObject),
                            module: onigWasmModule,
                        };
                    },
                });
            }
        }

        const { readFile } = await import("node:fs/promises");
        const { createRequire } = await import("node:module");
        const resolved = createRequire(import.meta.url).resolve("vscode-oniguruma/release/onig.wasm");

        return vscodeOniguruma.loadWASM(await readFile(resolved));
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
