import { makeLazy } from "@/utils/lazy";

import type { RegexEngine } from "shiki";
import { loadWASM, OnigScanner, OnigString } from "vscode-oniguruma";
import onigurumaWasmURL from "vscode-oniguruma/release/onig.wasm?url";
import type { IOnigLib } from "vscode-textmate";

export const loadOnigasmPromise = makeLazy(async () => {
    const res = await fetch(onigurumaWasmURL);

    return loadWASM(res);
});

export const createOnigurumaEngine = makeLazy(() => {
    return loadOnigasmPromise().then((): IOnigLib & RegexEngine => {
        return {
            createOnigScanner(sources) {
                return new OnigScanner(sources);
            },
            createOnigString(str) {
                return new OnigString(str);
            },
            createScanner(patterns) {
                return new OnigScanner(patterns.map((p) => (typeof p === "string" ? p : p.source)));
            },
            createString(s) {
                return new OnigString(s);
            },
        };
    });
});
