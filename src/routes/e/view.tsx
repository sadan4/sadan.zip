import { createFileRoute, redirect } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";

import { Explorer } from "./-ui";

import z from "zod";

const viewBundleParamsSchema = z.union([
    z.object({
        buildHash: z.string().default(""),
    }),
]);

export const Route = createFileRoute("/e/view")({
    component: ExplorerWrapper,
    validateSearch: zodValidator(viewBundleParamsSchema),
    beforeLoad({ search: { buildHash } }) {
        if (buildHash === "") {
            throw redirect({
                to: "/e",
            });
        }
    },
});

function ExplorerWrapper() {
    return <Explorer />;
}

