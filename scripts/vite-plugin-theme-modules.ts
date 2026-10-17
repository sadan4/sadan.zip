import { dedent } from "../src/utils/string.ts";

import { transform } from "lightningcss";
import { readFile } from "node:fs/promises";
import { basename, posix, relative } from "node:path";
import { normalizePath, type Plugin, type Rollup } from "vite";

const THEME_RE = /\.theme\.css$/;
// the browser requests the stylesheet at the dev url, and vite adds `?direct` because of the `Accept: text/css` header
const DIRECT_THEME_RE = /\.theme\.css\?(?:.*&)?direct\b/;
const VIRTUAL_PREFIX = "\0theme-module:";
// the suffix keeps vite:css and vite:css-post from treating the js module as css and overwriting it
const VIRTUAL_SUFFIX = ".js";
const VIRTUAL_RE = /^\0theme-module:/;

interface CompiledTheme {
    code: Uint8Array;
    className: string;
}

export function themeModules(): Plugin {
    let isDev = false;
    let root = "";
    let base = "/";

    function compile(ctx: Rollup.PluginContext, file: string, code: string): CompiledTheme {
        const { code: transformedCode, exports, warnings } = transform({
            code: Buffer.from(code),
            filename: file,
            // keeps the generated class name stable across machines and identical between the js module and the dev
            // stylesheet request
            projectRoot: root,
            cssModules: {
                pattern: isDev ? "[name]-[content-hash]" : undefined,
                pure: true,
            },
        });

        for (const warning of warnings) {
            ctx.warn(`[${warning.type} - ${warning.loc.filename}:${warning.loc.line}:${warning.loc.column}]: ${warning.message}`);
        }

        const themeExport = exports?.theme;

        if (!themeExport) {
            ctx.error(`css module ${JSON.stringify(file)} has no "theme" export`);
        }

        if (themeExport.composes.length) {
            ctx.error(`theme export from module ${JSON.stringify(file)} composes`);
        }

        return {
            code: transformedCode,
            className: themeExport.name,
        };
    }

    function toDevUrl(file: string): string {
        const path = file.startsWith(`${root}/`)
            ? posix.relative(root, file)
            : posix.join("@fs", file);

        return posix.join(base, path);
    }

    return {
        name: "vite-plugin-theme-modules",
        // resolveId has to run before vite:resolve claims the import
        enforce: "pre",
        configResolved(config) {
            isDev = config.command === "serve";
            root = normalizePath(config.root);
            base = config.base;
        },
        resolveId: {
            filter: {
                id: THEME_RE,
            },
            async handler(source, importer, options) {
                const resolved = await this.resolve(source, importer, {
                    ...options,
                    skipSelf: true,
                });

                if (!resolved || resolved.external) {
                    return resolved;
                }

                return `${VIRTUAL_PREFIX}${resolved.id}${VIRTUAL_SUFFIX}`;
            },
        },
        load: {
            filter: {
                id: VIRTUAL_RE,
            },
            async handler(id) {
                const file = normalizePath(id.slice(VIRTUAL_PREFIX.length, -VIRTUAL_SUFFIX.length));

                this.addWatchFile(file);

                const { code, className } = compile(this, file, await readFile(file, "utf8"));

                const href = isDev
                    ? toDevUrl(file)
                    // vite's asset placeholder, replaced in renderChunk with a base-aware url in every environment
                    : `__VITE_ASSET__${this.emitFile({
                        type: "asset",
                        name: basename(file),
                        originalFileName: normalizePath(relative(root, file)),
                        source: code,
                    })}__`;

                return dedent/*ts*/`
                    export const href = ${JSON.stringify(href)};

                    export const className = ${JSON.stringify(className)};
                `;
            },
        },
        transform: {
            filter: {
                id: DIRECT_THEME_RE,
            },
            handler(code, id) {
                if (!isDev) {
                    return;
                }

                const file = id.slice(0, id.indexOf("?"));
                const { code: transformedCode } = compile(this, file, code);

                return {
                    code: Buffer.from(transformedCode).toString("utf8"),
                    map: null,
                };
            },
        },
    };
}
