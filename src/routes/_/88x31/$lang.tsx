import _88x31Image from "@/assets/88x31.png";
import { Boilerplate } from "@/components/Boilerplate";
import { Clickable } from "@/components/Clickable";
import { Codeblock } from "@/components/Codeblock/Codeblock";
import { Box } from "@/components/layout/Box";
import { TabBar, TabBarPosition } from "@/components/layout/TabBar";
import { TextLink } from "@/components/Links";
import { Text } from "@/components/Text";
import { unreachable } from "@/utils/error";
import { loadGrammar, loadTheme } from "@/utils/shiki";
import { Language } from "@/utils/textmate";
import { TextmateTheme } from "@/utils/textmate/theme";
import { createFileRoute } from "@tanstack/react-router";

import htmlExampleContent from "./-sample.html?raw";
import _reactExampleContent from "./-sample.tsx?raw";

import { type ComponentProps } from "react";
import { z } from "zod";

const reactExampleContent = _reactExampleContent.trimEnd();

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
            className="size-88x31"
        >
            <img
                src={_88x31Image}
                className="size-88x31 pixelated"
            />
        </Clickable>
    );
}

const buttonPageParamsSchema = z.object({
    lang: z.enum(["html", "react"]).catch("html"),
});

type ButtonPageParams = z.infer<typeof buttonPageParamsSchema>;

export const Route = createFileRoute("/_/88x31/$lang")({
    component: ButtonPage,
    wrapInSuspense: true,
    params: {
        parse(rawParams): ButtonPageParams {
            return buttonPageParamsSchema.parse(rawParams);
        },
    },
    async loader({ params: { lang } }) {
        if (!import.meta.env.SSR) {
            switch (lang) {
                case "html": {
                    await Promise.all([loadGrammar(Language.HTML), loadTheme(TextmateTheme.TOKYO_NIGHT)]);
                    break;
                }
                case "react": {
                    await Promise.all([loadGrammar(Language.TYPESCRIPT_REACT), loadTheme(TextmateTheme.TOKYO_NIGHT)]);
                    break;
                }
                default:
                    unreachable();
            }
        }
    },
    staticData: {
        description: "My 88x31 button",
        imageUrl: "/assets/88x31.png",
        pageTitle: "88x31",
    },
});

// 88x31 button page
function ButtonPage() {
    const { lang } = Route.useParams();

    return (
        <>
            <Boilerplate />
            <div className="mt-4 flex w-full justify-center">
                <Box className="mr-2">
                    <Text
                        size="3xl"
                        color="primary"
                        center
                    >
                        Add My Button
                    </Text>
                    <Sadan88x31Button />
                    <TabBar
                        selectedTab={lang}
                        tabsPosition={TabBarPosition.LEFT}
                        tabs={[
                            {
                                id: "html",
                                RenderTab() {
                                    return (
                                        <TextLink
                                            to="/88x31/$lang"
                                            params={{ lang: "html" }}
                                        >
                                            HTML
                                        </TextLink>
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
                                        <TextLink
                                            to="/88x31/$lang"
                                            params={{ lang: "react" }}
                                        >
                                            React
                                        </TextLink>
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
