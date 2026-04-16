import { basename } from "node:path";
import { type Plugin } from "rollup";

const NATIVE_MODULE_REGEX = /\.node$/;
const queryRE = /\?.*$/s;
const hashRE = /#.*$/s;

function cleanUrl(url: string): string {
    return url.replace(hashRE, "").replace(queryRE, "");
}

export function copyNativeModules(): Plugin {
    return {
        name: "rollup-plugin-copy-native-modules",
        async load(id) {
            if (id[0] === "\0" || !NATIVE_MODULE_REGEX.test(id)) {
                return;
            }

            const assetId = this.emitFile({
                type: "asset",
                name: basename(id),
                source: await this.fs.readFile(cleanUrl(id)),
            });

            return /*js*/`
                import { createRequire } from "node:module";
                export default createRequire(import.meta.url)("./" + import.meta.ROLLUP_FILE_URL_${assetId});
            `;
        },
    };
}

export default copyNativeModules;
