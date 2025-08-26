import { Accordion, AccordionGroup, type AccordionGroupHandle } from "@/components/Accordion";
import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/Box";
import { Button } from "@/components/Button";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { HorizontalLine } from "@/components/HorizontalLine";
import { CheckedInput, Input, LabeledInput } from "@/components/Input";
import { TabBar } from "@/components/layout/TabBar";
import { Marquee } from "@/components/Marquee";
import { LabeledSwitch, Switch } from "@/components/Switch";
import { Text } from "@/components/Text";
import { LabeledTextArea, TextArea } from "@/components/TextArea";
import { keys } from "@/utils/array";
import cn, { buttonColors, textSize, textWeight } from "@/utils/cn";
import guh from "./guh.module.css"

import { useRef, useState } from "react";


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

function TabBarExample() {
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

function ButtonsExample() {
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

function SelectionExample() {
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
                <div className="flex items-center justify-between mb-4 gap-3">
                    <LabeledSwitch
                        value={ignoreInput}
                        onChange={setIgnoreInput}
                    >
                        Ignore User Input
                    </LabeledSwitch>
                    <Text
                        size="lg"
                    >
                        Controlled Switches
                    </Text>
                    <Button onClick={toggleAll}>
                        Toggle All Switches
                    </Button>
                </div>
                <div className="flex flex-row flex-wrap gap-4">
                    {switchValues.map((value, index) => (
                        <Switch
                            key={index}
                            value={value}
                            onChange={ignoreInput ? undefined : (newValue) => handleSwitchChange(index, newValue)}
                        />
                    ))}
                </div>
            </Box>
        </>
    );
}

function InputExample() {
    return (
        <>
            <Text
                size="xl"
                center
            >
                Input Field
            </Text>
            <div className="flex flex-col gap-2">
                <LabeledInput
                    onChange={() => {}}
                >
                    Input
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                >
                    With Placeholder
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                    initialValue="Initial Value"
                >
                    With Initial Value
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                    disabled
                >
                    Disabled
                </LabeledInput>
                <LabeledInput
                    onChange={() => { }}
                    placeholder="Placeholder"
                    initialValue="Read only Input"
                    readOnly
                >
                    Read Only
                </LabeledInput>
                <CheckedInput
                    check={() => false}
                    onValidChange={() => {}}
                >
                    Checked Input
                </CheckedInput>
                <CheckedInput
                    check={/fooba[rz]/}
                    onValidChange={() => {}}
                >
                    Regex Checked Input
                </CheckedInput>
                <CheckedInput
                    check={{
                        type: "len",
                        min: 3,
                        max: 10,
                    }}
                    onValidChange={() => {}}
                >
                    Length Checked Input
                </CheckedInput>
                <CheckedInput
                    check={/fooba[rz]/}
                    debounce={2000}
                    onValidChange={() => {}}
                >
                    2000s Debounced Check
                </CheckedInput>
                <CheckedInput
                    check={() => false}
                    errorMessage={() => (
                        <Text
                            size="sm"
                            color="error"
                            noselect
                        >
                            <Marquee>
                                Custom Error Message
                            </Marquee>
                        </Text>
                    )}
                    onValidChange={() => {}}
                >
                    Custom Error Message
                </CheckedInput>
                <CheckedInput
                    check={() => false}
                    onValidChange={() => { }}
                    disabled
                >
                    Disabled
                </CheckedInput>
            </div>
        </>
    );
}
function TextExample() {
    const [previewText, setPreviewText] = useState("");
    const [show, setShow] = useState(false);

    return (
        <>
            <div className="flex items-center justify-between mb-4">
                <Text
                    size="xl"
                    center
                >
                    TextComponents
                </Text>
                {
                    show && (
                        <Input
                            initialValue={previewText}
                            onChange={(e) => {
                                setPreviewText(e.target.value);
                            }}
                            placeholder="Preview Text"
                            className="!w-fit"
                        />
                    )
                }
                <Button onClick={() => setShow(!show)}>
                    {show ? "Hide Preview" : "Show Preview"}
                </Button>
            </div>
            {show && Object.keys(textSize)
                .flatMap((size) => {
                    return Object.keys(textWeight)
                        .map((weight) => (
                            <Text
                                weight={weight as any}
                                size={size as any}
                                key={`${weight}-${size}`}
                                tag="span"
                            >
                                {previewText || `${size}-${weight}`}
                                {previewText && (
                                    <Text
                                        size="md"
                                        tag="span"
                                    >
                                        {` (${size}-${weight})`}
                                    </Text>
                                )}
                            </Text>
                        ));
                })}
        </>
    );
}

function AccordionExample() {
    const group2Ref = useRef<AccordionGroupHandle>(null);

    return (
        <>
            <Text
                size="xl"
                center
            >
                Accordion
            </Text>
            <Accordion
                item={{
                    id: "sample-accordion",
                    render: () => <Text>Accordion Content</Text>,
                }}
            >
                <Text size="lg">Single Accordion Item</Text>
            </Accordion>
            <Text
                size="lg"
                className="my-2"
            >
                Accordion Group, Only One Open
            </Text>
            <AccordionGroup>
                <Box>
                    <Accordion
                        item={{
                            id: "1",
                            render() {
                                return <Text size="md">This is the content of Accordion 1.</Text>;
                            },
                        }}
                    >
                        <Text size="lg">Accordion 1</Text>
                    </Accordion>
                    <Accordion
                        item={{
                            id: "2",
                            render() {
                                return <Text size="md">{lorem}</Text>;
                            },
                        }}
                    >
                        <Text size="lg">Accordion 2</Text>
                    </Accordion>
                    <Accordion
                        item={{
                            id: "3",
                            render() {
                                return <Text size="md">This is the content of Accordion 3.</Text>;
                            },
                        }}
                    >
                        <Text size="lg">Accordion 3</Text>
                    </Accordion>
                </Box>
            </AccordionGroup>
            <div className="flex items-center justify-between my-2">
                <Text
                    size="lg"
                >
                    Accordion Group
                </Text>
                <Button
                    onClick={() => {
                        group2Ref.current?.closeAll();
                    }}
                >
                    Close All
                </Button>
            </div>
            <AccordionGroup
                onlyOneOpen={false}
                ref={group2Ref}
            >
                <Box>
                    <Accordion
                        item={{
                            id: "1",
                            render() {
                                return <Text size="md">This is the content of Accordion 1.</Text>;
                            },
                        }}
                    >
                        <Text size="lg">Accordion 1</Text>
                    </Accordion>
                    <Accordion
                        item={{
                            id: "2",
                            render() {
                                return <Text size="md">{lorem}</Text>;
                            },
                        }}
                    >
                        <Text size="lg">Accordion 2</Text>
                    </Accordion>
                    <Accordion
                        item={{
                            id: "3",
                            render() {
                                return <Text size="md">This is the content of Accordion 3.</Text>;
                            },
                        }}
                    >
                        <Text size="lg">Accordion 3</Text>
                    </Accordion>
                </Box>
            </AccordionGroup>
        </>
    );
}

function TextAreaExample() {
    return (
        <>
            <Text
                size="xl"
                center
            >
                Text Area
            </Text>
            <TextArea />
            <LabeledTextArea resize="both">
                Resizable
            </LabeledTextArea>
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
                        <TabBarExample />
                        <HorizontalLine className="my-4"/>
                        <ButtonsExample />
                        <HorizontalLine className="my-4" />
                        <SelectionExample />
                        <HorizontalLine className="my-4" />
                        <Text
                            size="xl"
                            center
                        >
                            scroll thingie
                        </Text>
                        <div className={guh.what}>
                            <Text
                                size="lg"
                                center
                            >
                                Black on hover
                            </Text>
                        </div>
                        <HorizontalLine className="my-4" />
                        <SelectionExample />
                        <HorizontalLine className="my-4" />
                        <SwitchExample />
                        <HorizontalLine className="my-4" />
                        <AccordionExample />
                        <HorizontalLine className="my-4" />
                        <InputExample />
                        <HorizontalLine className="my-4" />
                        <TextExample />
                        <HorizontalLine className="my-4" />
                        <TextAreaExample />
                    </Box>
                </div>
            </FooterContainer>
        </>
    );
}
