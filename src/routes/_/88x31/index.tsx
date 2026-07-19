import { createFileRoute, redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/_/88x31/")({
    beforeLoad() {
        // oxlint-disable-next-line typescript/only-throw-error
        throw redirect({
            to: "/88x31/$lang",
            params: { lang: "html" },
        });
    },
});
