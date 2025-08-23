import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/Box";
import { Button } from "@/components/Button";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { HorizontalLine } from "@/components/HorizontalLine";
import { TabBar } from "@/components/layout/TabBar";
import { Switch } from "@/components/Switch";
import { Text } from "@/components/Text";
import { keys } from "@/utils/array";
import cn, { buttonColors } from "@/utils/cn";

import { useState } from "react";


// cspell:disable
const lorem = `
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vestibulum ut dui est. Cras commodo, erat eget finibus varius, augue est dignissim turpis, nec bibendum nisl justo vitae sem. Aenean sit amet vulputate tortor. Nullam eu vestibulum nisi. Phasellus hendrerit sollicitudin malesuada. Nullam est tellus, convallis in justo quis, efficitur laoreet erat. Duis nulla elit, sodales sed vulputate faucibus, commodo quis sapien. Donec in ligula non risus sagittis fermentum nec ac diam. Phasellus vel dictum nisi, sed pharetra justo.

In viverra eleifend tortor ultricies molestie. Duis ullamcorper, lacus ac vehicula malesuada, tortor leo rhoncus enim, eu tempor ipsum purus non ipsum. Integer rutrum ipsum sit amet ante laoreet malesuada. Morbi hendrerit vestibulum neque in dignissim. Pellentesque aliquet tempor sem non ultricies. Nulla ac imperdiet erat, sit amet finibus lacus. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Etiam ornare accumsan tellus, ac aliquet turpis imperdiet et.
`;
// cspell:enable

function ManagedTabBar() {
    const [section, setSection] = useState<"1" | "2" | "3">("1");

    return (
        <>
            <TabBar
                selectedTab={section}
                tabs={[
                    {
                        id: "1",
                        render() {
                            return (
                                <Text size="md">
                                    This is the content of Tab 1.
                                </Text>
                            );
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
                                    <Text size="md">
                                        This is the content of Tab 2.
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
                            return (
                                <Text size="md">
                                    This is the content of Tab 3.
                                </Text>
                            );
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
            <div className="flex justify-between mt-4">
                <Button
                    onClick={() => setSection("1")}
                >
                    Tab 1
                </Button>
                <Button
                    onClick={() => setSection("2")}
                >
                    Tab 2
                </Button>
                <Button
                    onClick={() => setSection("3")}
                >
                    Tab 3
                </Button>
            </div>
        </>
    );
}

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
                        return <ManagedTabBar />;
                    },
                    renderTab() {
                        return (
                            <Text
                                size="md"
                                noselect
                            >
                                Managed TabBar
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

function SwitchExample() {
    const [switchValues, setSwitchValues] = useState<boolean[]>([true, false, true, false]);
    const [ignoreInput, setIgnoreInput] = useState(false);

    const toggleAll = () => {
        setSwitchValues((values) => values.map((v) => !v));
    };

    const handleSwitchChange = (index: number, newValue: boolean) => {
        setSwitchValues((values) => {
            const newValues = [...values];

            newValues[index] = newValue;
            return newValues;
        });
    };

    return (
        <>
            <Text
                size="xl"
                center
            >
                Switch
            </Text>
            <Box>
                <Text
                    size="lg"
                    center
                >
                    Uncontrolled Switches
                </Text>
                <div className="flex flex-row flex-wrap gap-4">
                    {Array.from({ length: 20 }, (_, i) => (
                        <Switch
                            key={i}
                            initialValue={Math.random() > 0.5}
                        />
                    ))}
                </div>
            </Box>

            <Box className="mt-8">
                <Text
                    size="lg"
                    center
                >
                    Controlled Switches
                </Text>
                <div className="flex items-center justify-center mb-4">
                    <Text
                        size="md"
                        className="mr-3"
                    >
                        Ignore User Input:
                    </Text>
                    <Switch
                        value={ignoreInput}
                        onChange={setIgnoreInput}
                    />
                </div>
                <div className="flex flex-row flex-wrap gap-4 mb-4">
                    {switchValues.map((value, index) => (
                        <Switch
                            key={index}
                            value={value}
                            onChange={ignoreInput ? undefined : (newValue) => handleSwitchChange(index, newValue)}
                        />
                    ))}
                </div>
                <div className="flex justify-center">
                    <Button onClick={toggleAll}>
                        Toggle All Switches
                    </Button>
                </div>
            </Box>
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
