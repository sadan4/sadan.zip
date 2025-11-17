import { makeLazy } from "@/utils/lazy";

import { loadWASM } from "onigasm";
import onigasmWasmURL from "onigasm/lib/onigasm.wasm?url";

export const loadOnigasmPromise = makeLazy(() => {
    return loadWASM(onigasmWasmURL);
});
