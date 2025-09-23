import { Button } from "@/components/Button";
import { HorizontalLine } from "@/components/HorizontalLine";
import { Text } from "@/components/Text";
import { lorem } from "@/utils/constants";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { TabBar } from ".";

import { useState } from "react";

const meta = {
    component: TabBar,
} satisfies Meta<typeof TabBar>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        tabs: [
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
                    return <Text>Tab 3 content</Text>;
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
        ],
    },
};

export const Controlled: Story = {
    args: {
        // unused
        tabs: [] as never,
    },
    render() {
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
                <div className="flex gap-3 mt-4">
                    <Button
                        colorType="outline"
                        onClick={() => setSection("1")}
                    >
                        Tab 1
                    </Button>
                    <Button
                        colorType="outline"
                        onClick={() => setSection("2")}
                    >
                        Tab 2
                    </Button>
                    <Button
                        colorType="outline"
                        onClick={() => setSection("3")}
                    >
                        Tab 3
                    </Button>
                </div>
            </>
        );
    },
};
