import _88x31Image from "@/assets/88x31.png";
import { Boilerplate } from "@/components/Boilerplate";
import { Clickable } from "@/components/Clickable";
import { Codeblock } from "@/components/Codeblock/Codeblock";
import { Box } from "@/components/layout/Box";
import { TabBar, TabBarPosition } from "@/components/layout/TabBar";
import { Text } from "@/components/Text";
import { Language } from "@/utils/textmate";
import { createFileRoute } from "@tanstack/react-router";

import htmlExampleContent from "./-sample.html?raw";
import reactExampleContent from "./-sample.tsx?raw";

import type { ComponentProps } from "react";

interface Sadan88x31ButtonProps extends Omit<ComponentProps<"a">, "href" | "rel"> {
}

// this file only exports components, eslint is stupid

function Sadan88x31Button(props: Sadan88x31ButtonProps) {
    return (
        <Clickable
            {...props}
            tag="a"
            href="/"
            rel="nofollow"
            style={{
                width: "88px",
                height: "31px",
            }}
        >
            <img
                src={_88x31Image}
                style={{
                    imageRendering: "pixelated",
                    width: "88px",
                    height: "31px",
                }}
            />
        </Clickable>
    );
}

// 88x31 button page
function ButtonPage() {
    return (
        <>
            <Boilerplate />
            <div className="mt-4 flex w-full justify-center">
                <Box className="mr-2 sm:w-full md:w-1/2">
                    <Text
                        size="3xl"
                        color="primary"
                        center
                    >
                        Add My Button
                    </Text>
                    <Sadan88x31Button />
                    <TabBar
                        tabsPosition={TabBarPosition.LEFT}
                        tabs={[
                            {
                                id: "html",
                                RenderTab() {
                                    return (
                                        <>
                                            HTML
                                        </>
                                    );
                                },
                                Render() {
                                    return (
                                        <Codeblock lang={Language.HTML}>
                                            {htmlExampleContent}
                                        </Codeblock>
                                    );
                                },
                            },
                            {
                                id: "react",
                                RenderTab() {
                                    return (
                                        <>
                                            React
                                        </>
                                    );
                                },
                                Render() {
                                    return (
                                        <Codeblock lang={Language.TYPESCRIPT_REACT}>
                                            {reactExampleContent}
                                        </Codeblock>
                                    );
                                },
                            },
                        ]}
                    />
                </Box>
            </div>
        </>
    );
}

export const Route = createFileRoute("/_layout/e/")();
