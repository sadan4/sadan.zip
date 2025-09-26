import { Boilerplate } from "@/components/Boilerplate";
import { Clickable } from "@/components/Clickable";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { Text } from "@/components/Text";
import { TextArea } from "@/components/TextArea";
import { copy } from "@/utils/clipboard";
import { demangleWords } from "@/utils/demangle";

import { useState } from "react";

export default function Demangler() {
    const [text, setText] = useState("");

    return (
        <>
            <Boilerplate />
            <FooterContainer
                footer={() => <DefaultFooter />}
            >
                <div className="flex h-full w-full flex-col items-center pt-[20vh]">
                    <Text
                        size="4xl"
                        color="accent"
                    >
                        Demangler
                    </Text>
                    <div className="mt-6 flex w-1/3 flex-col items-center gap-6">
                        <TextArea
                            size="lg"
                            value={text}
                            onChange={(e) => {
                                setText(e.target.value);
                            }}
                            placeholder="_ZN5sadan9demanglerB5cxx11Ev"
                            className="h-[45vh] max-h-[45vh] min-h-20 w-[60vw] max-w-[60vw] min-w-50 resize"
                        />
                        <div className="*:bg-accent-300 flex h-9 gap-3 *:flex *:items-center *:rounded-md *:px-2">
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
