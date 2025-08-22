import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/Box";
import { Button } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { HorizontalLine } from "@/components/HorizontalLine";
import { TabBar } from "@/components/layout/TabBar";
import { Text } from "@/components/Text";
import { keys } from "@/utils/array";
import cn, { buttonColors } from "@/utils/cn";
import { animated, useSpring } from "@react-spring/web";

import { useEffect, useReducer, useState } from "react";


// cspell:disable
const lorem = `
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum ut dui est. Cras commodo, erat eget finibus varius, augue est dignissim turpis, nec bibendum nisl justo vitae sem. Aenean sit amet vulputate tortor. Nullam eu vestibulum nisi. Phasellus hendrerit sollicitudin malesuada. Nullam est tellus, convallis in justo quis, efficitur laoreet erat. Duis nulla elit, sodales sed vulputate faucibus, commodo quis sapien. Donec in ligula non risus sagittis fermentum nec ac diam. Phasellus vel dictum nisi, sed pharetra justo.

In viverra eleifend tortor ultricies molestie. Duis ullamcorper, lacus ac vehicula malesuada, tortor leo rhoncus enim, eu tempor ipsum purus non ipsum. Integer rutrum ipsum sit amet ante laoreet malesuada. Morbi hendrerit vestibulum neque in dignissim. Pellentesque aliquet tempor sem non ultricies. Nulla ac imperdiet erat, sit amet finibus lacus. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam ornare accumsan tellus, ac aliquet turpis imperdiet et.
`;
// cspell:enable

function ExampleTabBar() {
    return (
        <>
            <Text
                size="xl"
                center
            >Tab Bar
            </Text>
            <TabBar tabs={[
                {
                    id: "1",
                    render() {
                        return <Text size="md">Tab 1 content</Text>;
                    },
                    renderTab() {
                        return (
                            <Text
                                size="md"
                                noselect
                            >
                                Tab 1
                            </Text>
                        );
                    },
                },
                {
                    id: "2",
                    render() {
                        return (
                            <div>
                                <Text
                                    size="md"
                                    center
                                >
                                    Tab 2 content
                                </Text>
                                <HorizontalLine color="white-700" />
                                <Text size="sm" >
                                    {lorem}
                                </Text>
                            </div>
                        );
                    },
                    renderTab() {
                        return (
                            <Text
                                size="md"
                                noselect
                            >
                                Tab 2
                            </Text>
                        );
                    },
                },
                {
                    id: "3",
                    render() {
                        return <Text size="md">Tab 3 content</Text>;
                    },
                    renderTab() {
                        return (
                            <Text
                                size="md"
                                noselect
                            >
                                Tab 3
                            </Text>
                        );
                    },
                },
            ]}
            />
        </>
    );
}

function ExampleButtons() {
    const border = false;
    // const [border, setBorder] = useState(false);


    return (
        <>
            <Text
                size="xl"
                center
            >
                Buttons
            </Text>
            <div className={cn("flex gap-2 flex-wrap", border && "*:border *:border-red-500")}>
                {keys(buttonColors)
                    .map((color) => {
                        return (
                            <Button
                                key={color}
                                color={color}
                            >
                                Click Me!
                            </Button>
                        );
                    })}
            </div>
        </>
    );
}

function SelectionDropDown() {
    return (
        <>
            <Text
                size="xl"
                center
            >
                Selection Menu
            </Text>
            <Text size="xl">
                TODO
            </Text>
        </>
    );
}

enum SwitchState {
    OFF,
    ON,
    HELD,
}

function xFromSwitchState(state: SwitchState): number {
    switch (state) {
        case SwitchState.OFF:
            return 6;
        case SwitchState.ON:
            return 18;
        case SwitchState.HELD:
            return 12;
        default: {
            throw new Error("unhandled state");
        }
    }
}

function SwitchExample() {
    const [enabled, toggleEnabled] = useReducer((x) => !x, false);
    const [state, setState] = useState(SwitchState.OFF);

    const [{ cx }, switchPosApi] = useSpring(() => ({
        cx: xFromSwitchState(state),
        config: {
            mass: 0.5,
        },
    }));

    useEffect(() => {
        switchPosApi.start({
            cx: xFromSwitchState(state),
        });
    }, [state, switchPosApi]);

    return (
        <>
            <Text
                size="xl"
                center
            >
                Switch
            </Text>
            <div>
                <Clickable
                    className={cn("w-12 h-8 rounded-full flex justify-center transition-colors duration-250", enabled ? "bg-primary" : "bg-bg-300")}
                    onMouseDown={(e) => {
                        // stop random other text from being selected
                        if (e.detail > 1) {
                            e.preventDefault();
                        }
                        setState(SwitchState.HELD);
                    }}
                    onMouseUp={() => {
                        if (state !== SwitchState.HELD) {
                            return;
                        }
                        if (enabled) {
                            setState(SwitchState.OFF);
                        } else {
                            setState(SwitchState.ON);
                        }
                        toggleEnabled();
                    }}
                    onMouseLeave={() => {
                        if (enabled) {
                            setState(SwitchState.ON);
                        } else {
                            setState(SwitchState.OFF);
                        }
                    }}
                >
                    <svg
                        viewBox="0 0 24 24"
                        className="flex items-center justify-center w-full h-full"
                    >
                        <animated.circle
                            cx={cx}
                            cy={12}
                            r={8}
                            className={cn("transition-colors duration-250", enabled ? "fill-neutral" : "fill-bg-fg-600")}
                        />
                    </svg>
                </Clickable>
            </div>
        </>
    );
}

export default function Components() {
    return (
        <>
            <Boilerplate />
            <FooterContainer
                footer={() => <DefaultFooter />}
            >
                <div className="mt-4 flex flex-col items-center">
                    <Text
                        size="4xl"
                        weight="extraBold"
                    >
                        Component Testing
                    </Text>
                    <Box className="mt-6 w-[40vw]">
                        <ExampleTabBar />
                        <HorizontalLine className="my-4"/>
                        <ExampleButtons />
                        <HorizontalLine className="my-4" />
                        <SelectionDropDown />
                        <HorizontalLine className="my-4" />
                        <SwitchExample />
                    </Box>
                </div>
            </FooterContainer>
        </>
    );
}
