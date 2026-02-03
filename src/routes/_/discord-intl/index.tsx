import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { AnimateHeight } from "@/components/effects/AnimateHeight";
import { LabeledInput } from "@/components/Input";
import { Box } from "@/components/layout/Box/Box";
import { HorizontalLine } from "@/components/Lines/HorizontalLine";
import { Text } from "@/components/Text";
import { copy } from "@/utils/clipboard";
import cn from "@/utils/cn";
import { createFileRoute } from "@tanstack/react-router";

import mappings from "./key-mappings.json";

import { useState } from "react";

export const Route = createFileRoute("/_/discord-intl/")({
    component: DiscordIntlLookup,
    staticData: {
        description: "Map hashed discord intl keys to their original strings.",
        pageTitle: "Discord Intl",
    },
});


function DiscordIntlLookup() {
    const [search, setSearch] = useState("");
    const result: string | null = (mappings as Record<string, string>)[search] || null;

    return (
        <>
            <Boilerplate />
            <div className="mt-6 flex justify-center">
                <Box className="w-1/2">
                    <Text
                        size="2xl"
                        center
                    >
                        Discord Intl Lookup
                    </Text>
                    <HorizontalLine className="my-2" />
                    <div className="flex w-1/2 flex-col gap-3 self-center">
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
                            <div className="flex flex-col gap-2">
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
                                <div className={cn("flex items-center justify-between transition-colors")}>
                                    <Text
                                        color={result != null ? "success" : "error"}
                                        className="max-w-6/10 overflow-clip text-nowrap"
                                    >
                                        #{"{"}intl::{result ?? "No result :("}{"}"}
                                    </Text>
                                    {
                                        result
                                        && (
                                            <Button onClick={() => {
                                                copy(`#{intl::${result}}`);
                                            }}
                                            >
                                                Copy Vencord Find
                                            </Button>
                                        )
                                    }
                                </div>
                            </div>
                        </AnimateHeight>
                    </div>
                </Box>
            </div>
        </>
    );
}

