import type { Meta, StoryObj } from "@storybook/react-vite";

import { TextArea } from ".";

const meta = {
    component: TextArea,
} satisfies Meta<typeof TextArea>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        children: "Text Area",
    },
};

export const Resizable: Story = {
    args: {
        resize: "both",
    },
};
