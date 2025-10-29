import express from "express";
import { extname } from "node:path";
import * as zlib from "node:zlib";
import type { ViteDevServer } from "vite";
import getPort, {portNumbers} from "get-port";

const isTest = process.env.NODE_ENV === 'test' || !!process.env.VITE_TEST_BUILD

interface CreateServerOptions {
    root?: string;
    isProd?: boolean;
    hmrPort?: number;
}

async function createServer({isProd = process.env.NODE_ENV === "production", root = process.cwd(), hmrPort}: CreateServerOptions = {}) {
    const isDev = !isProd;
    const app = express();
    let vite: ViteDevServer | undefined;
    if (isDev) {
        vite = await (await import("vite")).createServer({
            root,
            logLevel: isTest ? "error" : "info",
            server: {
                middlewareMode: true,
                watch: {
                    // During tests we edit the files too fast and sometimes chokidar
                    // misses change events, so enforce polling for consistency
                    usePolling: true,
                    interval: 100,
                },
                hmr: {
                    port: hmrPort,
                }
            },
            appType: "custom",
        });
        app.use(vite.middlewares)
    } else {
        app.use(
        (await import('compression')).default({
            brotli: {
            flush: zlib.constants.BROTLI_OPERATION_FLUSH,
            },
            flush: zlib.constants.Z_SYNC_FLUSH,
        }),
        )
    }
    if (isProd) {
        app.use(express.static("./dist/client"))
    }

    app.use("*", async (req, res) => {
        try {
            const url = req.originalUrl

            if (extname(url) !== '') {
                console.warn(`${url} is not valid router path`)
                res.status(404)
                res.end(`${url} is not valid router path`)
                return
            }

            // Best effort extraction of the head from vite's index transformation hook
            let viteHead = isDev
                ? await vite!.transformIndexHtml(
                    url,
                    `<html><head></head><body></body></html>`,
                )
                : ''

            viteHead = viteHead.substring(
                viteHead.indexOf('<head>') + 6,
                viteHead.indexOf('</head>'),
            )

            const entry = await (async () => {
                if (isDev) {
                    return vite!.ssrLoadModule('/src/server.tsx')
                } else {
                    //@ts-expect-error
                    return import('./dist/server/server.js')
                }
            })()

            console.info('Rendering: ', url, '...')
            entry.render({ req, res, head: viteHead })
        } catch (e: any) {
            if (isDev) {
                vite!.ssrFixStacktrace(e)
            }
            console.info(e.stack)
            res.status(500).end(e.stack)
        }
    });

    return {app, vite};
}

if (!isTest) {
    createServer().then(async ({app}) => {
        const port = await getPort({port: portNumbers(5173, 5273)});
        app.listen(port, () => {
            console.log(`Server running at http://localhost:${port}/`);
        })
    })
}
