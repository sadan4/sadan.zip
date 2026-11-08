import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/layout/Box";
import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/_/blog")({
    component: BlogLayout,
});

function BlogLayout() {
    return (
        <>
            <Boilerplate />
            <div className="mx-auto w-full p-4 pt-8 md:w-3/4 md:px-0 lg:w-1/2">
                <Box>
                    <Outlet />
                </Box>
            </div>
        </>
    );
}
