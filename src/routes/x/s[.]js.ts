import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/x/s.js")({
    server: {
        handlers: {
            async GET() {
                const res = await fetch("https://static.cloudflareinsights.com/beacon.min.js");

                return new Response(res.body, {
                    status: res.status,
                    headers: {
                        "content-type": "application/javascript; charset=utf-8",
                        "cache-control": "public, max-age=86400",
                    },
                });
            },
        },
    },
});
