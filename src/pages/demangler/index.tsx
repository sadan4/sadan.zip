import { Boilerplate } from "@/components/Boilerplate";
import { DefaultFooter, FooterContainer } from "@/components/Footer";

export default function Demangler() {
    return (
        <>
            <Boilerplate />
            <FooterContainer
                footer={() => <DefaultFooter />}
            >
                <div className="flex h-full w-full items-center flex-col pt-52">
                    <div className="text-4xl text-accent">
                        Demangler
                    </div>
                    <div className="text-lg text-secondary mt-4">
                        This page is under construction. Please check back later.
                    </div>
                </div>
            </FooterContainer>
        </>
    );
}
