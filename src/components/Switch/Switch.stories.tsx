import type { Meta, StoryObj } from "@storybook/react-vite";

import { Switch } from ".";

const meta = {
    component: Switch,
} satisfies Meta<typeof Switch>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
    },
};

export const ControlledTrue: Story = {
    args: {
        value: true,
    },
};

export const ControlledFalse: Story = {
    args: {
        value: false,
    },
};
