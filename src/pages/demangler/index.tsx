import { Boilerplate } from "@/components/Boilerplate";
import { Clickable } from "@/components/Clickable";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { Text } from "@/components/Text";
import { TextArea } from "@/components/TextArea";
import { copy } from "@/utils/clipboard";
import { demangleWords } from "@/utils/demangle";

import { useState } from "react";

interface DemangleOutputProps {
    text: string;
}

function DemangleOutput({ text }: DemangleOutputProps) {
    return <Text>{text}</Text>;
}
export default function Demangler() {
    const [text, setText] = useState("");

    return (
        <>
            <Boilerplate />
            <FooterContainer
                footer={() => <DefaultFooter />}
            >
                <div className="flex h-full w-full items-center flex-col pt-[20vh]">
                    <Text
                        size="4xl"
                        color="accent"
                    >
                        Demangler
                    </Text>
                    <div className="flex flex-col items-center gap-6 mt-6 w-1/3">
                        <TextArea
                            size="lg"
                            value={text}
                            onChange={(e) => {
                                setText(e.target.value);
                            }}
                            placeholder="_ZN5sadan9demanglerB5cxx11Ev"
                            className="min-w-50 min-h-20 w-[60vw] max-w-[60vw] h-[45vh] max-h-[45vh] resize"
                        />
                        <div className="flex gap-3 *:flex *:rounded-md *:bg-accent h-9 *:px-2 *:items-center">
                            <Clickable
                                onClick={() => {
                                    if (!text) {
                                        return;
                                    }
                                    setText(demangleWords(text));
                                }}
                            >
                                <Text
                                    tag="span"
                                    color="black-300"
                                    noselect
                                    nowrap
                                >
                                    Demangle
                                </Text>
                            </Clickable>
                            <Clickable
                                onClick={() => {
                                    if (text) {
                                        copy(text);
                                    }
                                }}
                            >
                                <Text
                                    tag="span"
                                    color="black-300"
                                    noselect
                                    nowrap
                                >
                                    Copy To Clipboard
                                </Text>
                            </Clickable>
                        </div>
                    </div>
                </div>
            </FooterContainer>
        </>
    );
}
