import { DefaultFooter } from "@/components/Footer";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/_")({
    component: RouteComponent,
});

function RouteComponent() {
    return (
        <div className="size-full">
            <ScrollArea className="h-full">
                <div className="grid size-full grid-rows-[1fr_min-content]">
                    <div>
                        <Outlet />
                    </div>
                    <div className="flex justify-center">
                        <DefaultFooter />
                    </div>
                </div>
            </ScrollArea>
        </div>
    );
}
