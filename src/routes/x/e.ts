import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/x/e")({
    server: {
        handlers: {
            async POST({ request }) {
                const origin = request.headers.get("origin") ?? new URL(request.url).origin;

                const res = await fetch("https://cloudflareinsights.com/cdn-cgi/rum", {
                    method: "POST",
                    body: request.body,
                    headers: {
                        "content-type": request.headers.get("content-type") ?? "application/json",
                        "user-agent": request.headers.get("user-agent") ?? "",
                        "x-forwarded-for": request.headers.get("cf-connecting-ip") ?? "",
                        origin,
                        referer: request.headers.get("referer") ?? `${origin}/`,
                    },
                });

                return new Response(null, { status: res.status });
            },
        },
    },
});
