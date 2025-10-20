import type { Meta, StoryObj } from "@storybook/react-vite";

import { CheckedInput } from ".";
import { Marquee } from "../Marquee";
import { Text } from "../Text";

const meta = {
    component: CheckedInput,
} satisfies Meta<typeof CheckedInput>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        check() {
            return false;
        },
    },
};

export const RegexCheck: Story = {
    args: {
        check: /fooba[rz]/,
    },
};

export const LengthCheck: Story = {
    args: {
        check: {
            type: "len",
            min: 3,
            max: 10,
        },
    },
};

export const DebouncedCheck: Story = {
    name: "2000 ms Debounced Check",
    args: {
        check: /fooba[rz]/,
        debounce: 2000,
    },
};

export const CustomErrorMessage: Story = {
    args: {
        check() {
            return false;
        },
        errorMessage() {
            return (
                <Text
                    size="sm"
                    color="error"
                    noselect
                >
                    <Marquee>
                        Custom Error Message
                    </Marquee>
                </Text>
            );
        },
    },
};

export const Disabled: Story = {
    args: {
        check() {
            return false;
        },
        disabled: true,
        initialValue: "Inital Value",
    },
};
