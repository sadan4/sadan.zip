import { makeDemangler } from "@sadan4/demangler/wasm";
import wasmBundleUrl from "@sadan4/demangler/wasm/compiled.wasm?url";

const demangler = await makeDemangler(wasmBundleUrl);

export function demangle(mangled: string): string {
    return demangler.demangle(mangled) ?? mangled;
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

