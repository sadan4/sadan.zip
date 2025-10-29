import { DefaultFooter, FooterContainer, FooterContent, FooterFooter } from "@/components/Footer";
import { createFileRoute, Outlet } from "@tanstack/react-router";

export const Route = createFileRoute("/_layout")({
    component: LayoutComponent,
});

function LayoutComponent() {
    return (
        <div className="h-full w-full">
            <FooterContainer>
                <FooterContent>
                    <Outlet />
                </FooterContent>
                <FooterFooter>
                    <DefaultFooter />
                </FooterFooter>
            </FooterContainer>
        </div>
    );
}
