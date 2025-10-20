import type { Meta, StoryObj } from "@storybook/react-vite";

import { Input } from ".";

const meta = {
    component: Input,
} satisfies Meta<typeof Input>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
};

export const WithPlaceholder: Story = {
    args: {
        placeholder: "Placeholder Text...",
    },
};

export const WithInitialValue: Story = {
    args: {
        initialValue: "Initial Value",
    },
};

export const WithClearButton: Story = {
    args: {
        clearButton: true,
        initialValue: "Initial Value",
    },
};

export const Disabled: Story = {
    args: {
        disabled: true,
        placeholder: "Placeholder",
    },
};

export const ReadOnly: Story = {
    args: {
        readOnly: true,
        initialValue: "Read Only Value",
    },
};

