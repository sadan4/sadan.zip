import {
    createRequestHandler,
    renderRouterToStream,
    RouterServer,
} from "@tanstack/react-router/ssr/server";

import { makeRouter } from "./router";

import type express from "express";
import { pipeline } from "node:stream/promises";

export async function render({
    req,
    res,
    cssPath,
}: {
    cssPath: string;
    req: express.Request;
    res: express.Response;
}) {
    // Convert the express request to a fetch request
    const url = new URL(req.originalUrl || req.url, "http://localhost:3000").href;

    const request = new Request(url, {
        method: req.method,
        headers: (() => {
            const headers = new Headers();

            for (const [key, value] of Object.entries(req.headers)) {
                headers.set(key, value as any);
            }
            return headers;
        })(),
    });

    // Create a request handler
    const handler = createRequestHandler({
        request,
        createRouter: () => {
            const router = makeRouter();

            // Update each router instance with the head info from vite
            router.update({
                context: {
                    ...router.options.context,
                    cssPath,
                },
            });
            return router;
        },
    });

    // Let's use the default stream handler to create the response
    const response = await handler(({ request, responseHeaders, router }) => {
        return renderRouterToStream({
            request,
            responseHeaders,
            router,
            children: <RouterServer router={router} />,
        });
    });

    // Convert the fetch response back to an express response
    res.statusMessage = response.statusText;
    res.status(response.status);

    response.headers.forEach((value, name) => {
        res.setHeader(name, value);
    });

    // Stream the response body
    return pipeline(response.body as any, res);
}
