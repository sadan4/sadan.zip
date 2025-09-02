import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/Box";
import { Button } from "@/components/Button";
import { AnimateHeight } from "@/components/effects/AnimateHeight";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { HorizontalLine } from "@/components/HorizontalLine";
import { LabeledInput } from "@/components/Input";
import { Text } from "@/components/Text";
import { copy } from "@/utils/clipboard";
import cn from "@/utils/cn";

import mappings from "./keys-mapping.json";

import { useState } from "react";

export default function DiscordIntlLookup() {
    const [search, setSearch] = useState("");
    const result: string | null = mappings[search] || null;

    return (
        <>
            <Boilerplate />
            <FooterContainer footer={() => <DefaultFooter />}>
                <div className="flex justify-center mt-6">
                    <Box className="w-1/2">
                        <Text
                            size="2xl"
                            center
                        >
                            Discord Intl Lookup
                        </Text>
                        <HorizontalLine className="my-2" />
                        <div className="flex flex-col gap-3 w-1/2 self-center">
                            <LabeledInput
                                placeholder="nkq1l5"
                                onChange={({ target: { value } }) => {
                                    if (value.length < 3) {
                                        setSearch("");
                                    } else {
                                        setSearch(value);
                                    }
                                }}
                                onClear={() => {
                                    setSearch("");
                                }}
                                clearButton
                            >
                                Enter a hashed Discord intl key
                            </LabeledInput>
                            <AnimateHeight
                                show={!!search}
                            >
                                <div className={cn("flex items-center justify-between transition-colors")}>
                                    <Text
                                        color={result != null ? "success" : "error"}
                                    >
                                        {result ?? "No result :("}
                                    </Text>
                                    {
                                        result
                                        && (
                                            <Button onClick={() => {
                                                copy(result);
                                            }}
                                            >
                                                Copy to Clipboard
                                            </Button>
                                        )
                                    }
                                </div>
                            </AnimateHeight>
                        </div>
                    </Box>
                </div>
            </FooterContainer>
        </>
    );
}
