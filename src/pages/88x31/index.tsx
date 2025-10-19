import _88x31Image from "@/assets/88x31.png";
import { Boilerplate } from "@/components/Boilerplate";
import { Clickable } from "@/components/Clickable";
import { CodeblockLang } from "@/components/Codeblock";
import { Codeblock } from "@/components/Codeblock/Codeblock";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { Box } from "@/components/layout/Box";
import { TabBar } from "@/components/layout/TabBar";
import { TabBarPosition } from "@/components/layout/TabBar/enum";
import { Text } from "@/components/Text";

import htmlExampleContent from "./_sample.html?raw";
import reactExampleContent from "./_sample.tsx?raw";

import type { ComponentProps } from "react";

interface Sadan88x31ButtonProps extends Omit<ComponentProps<"a">, "href" | "rel"> {
}

// this file only exports components, eslint is stupid
// eslint-disable-next-line react-refresh/only-export-components
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

export default function Component_88x31_Button() {
    return (
        <>
            <Boilerplate />
            <FooterContainer footer={() => <DefaultFooter />} >
                <div className="mt-4 flex w-full justify-center">
                    <Box className="w-1/2">
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
                                            <Codeblock lang={CodeblockLang.HTML}>
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
                                            <Codeblock lang={CodeblockLang.TSX}>
                                                {reactExampleContent}
                                            </Codeblock>
                                        );
                                    },
                                },
                            ]}
                        />
                    </Box>
                </div>
            </FooterContainer>
        </>
    );
}
