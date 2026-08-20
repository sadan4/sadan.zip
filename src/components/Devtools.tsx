import { useIsClient } from "@/hooks/isClient";

import { lazy, Suspense } from "react";

// the devtools are solid based, and solid resolves to its ssr build inside the
// worker environment, which is missing exports the devtools use. keeping the
// import inside a branch that is dead in the ssr and production builds keeps it
// out of both bundles
const LazyDevtools = import.meta.env.SSR || !import.meta.env.DEV
    ? null
    : lazy(async () => {
        const [{ TanStackDevtools }, { TanStackRouterDevtoolsPanel }] = await Promise.all([
            import("@tanstack/react-devtools"),
            import("@tanstack/react-router-devtools"),
        ]);

        return {
            default: () => (
                <TanStackDevtools
                    config={{
                        position: "bottom-right",
                    }}
                    plugins={[
                        {
                            name: "Tanstack Router",
                            render: <TanStackRouterDevtoolsPanel />,
                        },
                    ]}
                />
            ),
        };
    });

export function Devtools() {
    // the devtools are client only, so they must not render on the first
    // (hydrating) render either
    const isClient = useIsClient();

    if (!isClient || !LazyDevtools) {
        return null;
    }

    return (
        <Suspense>
            <LazyDevtools />
        </Suspense>
    );
}
