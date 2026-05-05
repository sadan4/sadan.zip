import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_/")({
    beforeLoad(_ctx) {
        const redir = redirect({
            to: "/{-$tab}",
            params: { tab: "about" },
            // Tanstack router is buggy and incorrectly sets the headers
            // headers: { Location: "/about" },
            statusCode: 302,
        });

        console.log({
            redir,
            opts: redir.options,
        });

        throw redir;
    },
    staticData: {
        pageTitle: "Home",
        description: "My silly website.",
        imageUrl: "/assets/avatar.webp",
    },
});

