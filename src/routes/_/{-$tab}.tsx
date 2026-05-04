import { HomePage } from "@/components/HomePage";
import { createFileRoute, notFound } from "@tanstack/react-router";

import * as z from "zod";

const tabParamSchema = z.object({
    tab: z.enum(["about", "links"]),
});

export const Route = createFileRoute("/_/{-$tab}")({
    params: {
        parse(rawParams) {
            try {
                return tabParamSchema.parse(rawParams);
            } catch (e) {
                if (e instanceof z.ZodError) {
                    throw notFound();
                } else {
                    throw e;
                }
            }
        },
    },
    component: RouteComponent,
});

function RouteComponent() {
    const { tab } = Route.useParams();

    return <HomePage tab={tab} />;
}
