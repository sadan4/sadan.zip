import type { Plugin, ResolvedConfig } from "vite";
import { bundleAsync, transform } from "lightningcss";

export function themeModules(): Plugin {
    let isDev = false;
    return {
        name: "vite-plugin-theme-modules",
        configResolved(config) {
            isDev = config.command === "serve";
        },
        transform: {
            filter: {
                id: /\.theme\.css$/
            },
            handler(code, id, options) {
                const { code: transformedCode, dependencies, exports, map, warnings } = transform({
                    code: Uint8Array.from(code),
                    filename: id,
                    cssModules: true
                })
            },
        }
    }
}