import { Button } from "@/components/Button";
import { Text } from "@/components/Text";
import { lorem } from "@/utils/constants";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { Accordion, AccordionGroup, type AccordionGroupHandle } from ".";

import { useRef } from "react";

const meta = {
    component: Accordion,
    subcomponents: {
        AccordionGroup,
    },
} satisfies Meta<typeof Accordion>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        item: {
            id: "item-1",
            render() {
                return <Text>Single Accordion Content</Text>;
            },
        },
        children: <Text>Single Accordion</Text>,
    },
};

export const OnlyOneOpen: Story = {
    args: {
        // never used
        item: {} as never,
    },
    render() {
        return (
            <AccordionGroup>
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
            </AccordionGroup>
        );
    },
};

export const CloseAllButton: Story = {
    args: {
        // never used
        item: {} as never,
    },
    render() {
        const ref = useRef<AccordionGroupHandle>(null);

        return (
            <>
                <div className="my-2 flex items-center justify-between">
                    <Text
                        size="lg"
                    >
                        Accordion Group
                    </Text>
                    <Button
                        onClick={() => {
                            ref.current?.closeAll();
                        }}
                    >
                        Close All
                    </Button>
                </div>
                <AccordionGroup
                    onlyOneOpen={false}
                    ref={ref}
                >
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
                </AccordionGroup>
            </>

        );
    },
};
