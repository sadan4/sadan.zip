import { makeDemangler } from "@sadan4/demangler/wasm";
import wasmBundleUrl from "@sadan4/demangler/wasm/compiled.wasm?url";

import { error } from "./error";


// FIXME: fix the exports for @sadan4/demangler to export an interface instead of a class
type Demangler = Awaited<ReturnType<typeof makeDemangler>>;

const demangler: Demangler = (!import.meta.env.SSR as never) && await makeDemangler(wasmBundleUrl);

export function demangle(mangled: string): string {
    if (import.meta.env.SSR) {
        error("demangle() called in SSR environment");
    } else {
        return demangler.demangle(mangled) ?? mangled;
    }
}

export function demangleWords(mangled: string): string;
export function demangleWords(mangled: string[]): string[];
export function demangleWords(mangled: string | string[]): string | string[] {
    if (Array.isArray(mangled)) {
        return mangled.map(demangle);
    }
    return mangled.split("\n")
        .map((mangled) => mangled
            .split(" ")
            .map(demangle)
            .join(" "))
        .join("\n");
}

