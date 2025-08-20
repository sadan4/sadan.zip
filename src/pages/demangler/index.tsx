import { Boilerplate } from "@/components/Boilerplate";
import { Clickable } from "@/components/Clickable";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { Input } from "@/components/Input";
import { makeDemangler } from "@sadan4/demangler/wasm";
import wasmBundleUrl from "@sadan4/demangler/wasm/compiled.wasm?url";

import { useState } from "react";

const demangler = await makeDemangler(wasmBundleUrl);

interface DemangleOutputProps {
    text: string;
}

function DemangleOutput({ text }: DemangleOutputProps) {
    return (
        <div>
            {text}
        </div>
    );
}
export default function Demangler() {
    const [text, setText] = useState("");
    const [output, setOutput] = useState("");

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
                    <div className="flex items-center gap-6 mt-6">
                        <Input
                            textSize="lg"
                            value={text}
                            onChange={(e) => {
                                setText(e.target.value);
                            }}
                            placeholder="_ZN5sadan9demanglerB5cxx11Ev"
                            className="w-100"
                        />
                        <Clickable
                            className="bg-accent text-bg-300 p-2 rounded-md"
                            onClick={() => {
                                if (!text) {
                                    setOutput("No Input :(");
                                    return;
                                }
                                setOutput(demangler.demangle(text) ?? "Unable to demangle input :(");
                            }}
                        >
                            Demangle It
                        </Clickable>
                    </div>
                    <div className="mt-6" />
                    {
                        output && <DemangleOutput text={output} />
                    }
                </div>
            </FooterContainer>
        </>
    );
}
