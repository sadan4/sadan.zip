import { DefaultFooter } from "@/components/Footer";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/_layout")({
    component: LayoutComponent,
});

function LayoutComponent() {
    return (
        <div className="h-full w-full">
            <ScrollArea className="h-full max-h-full">
                <div className="grid h-full w-full grid-rows-[1fr_min-content]">
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
