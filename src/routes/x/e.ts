import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/x/e")({
    server: {
        handlers: {
            async POST({ request }) {
                const res = await fetch("https://cloudflareinsights.com/cdn-cgi/rum", {
                    method: "POST",
                    body: request.body,
                    headers: {
                        "content-type": request.headers.get("content-type") ?? "application/json",
                        "user-agent": request.headers.get("user-agent") ?? "",
                        "x-forwarded-for": request.headers.get("cf-connecting-ip") ?? "",
                    },
                });

                return new Response(null, { status: res.status });
            },
        },
    },
});
