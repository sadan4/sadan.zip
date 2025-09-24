import type { Meta, StoryObj } from "@storybook/react-vite";

import { Select, type SelectOption } from ".";
import { Marquee } from "../Marquee";
import { Text } from "../Text";

import { useState } from "react";
import cn from "@/utils/cn";

const meta = {
    args: {
        items: [
            {
                label: "foo",
                value: "foo",
                key: "foo",
                typedValue: "foo",
            },
            {
                label: "bar",
                value: "bar",
                key: "bar",
                typedValue: "bar",
            },
            {
                label: "baz",
                value: "baz",
                key: "baz",
                typedValue: "baz",
            },
        ] as const satisfies SelectOption<PropertyKey>[],
        scrollAreaClassName: "max-h-60",
    },
    component: Select,
    render(args) {
        const [currentValue, setCurrentValue] = useState<any>();

        return (
            <div>
                <Select
                    {...args}
                    onChange={(value) => setCurrentValue(value)}
                />
                <Text>Current Value: {currentValue}</Text>
            </div>
        );
    },
} satisfies Meta<typeof Select>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
};

export const StayOpenAfterSelect: Story = {
    args: {
        closeOnSelect: false,
    },
};
export const ManyItems: Story = {
    args: {
        items: Array.from({ length: 100 }, (_, i) => ({
            label: `Item ${i + 1}`,
            value: `item${i + 1}`,
            key: `item${i + 1}`,
            typedValue: `item ${i + 1}`,
        })) satisfies SelectOption<PropertyKey>[],
    },
};

export const CustomLabel: Story = {
    args: {
        customChildren: true,
        children: <Marquee className="w-fit">Custom Label</Marquee>,
    },
};
