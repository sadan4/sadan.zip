import { Button } from "@/components/Button";
import type { Meta, StoryObj } from "@storybook/react-vite";


const meta = {
    component: Button,
    args: {
        disabled: false,
        wrap: false,
    },
    argTypes: {
        disabled: {
            control: "boolean",
        },
        wrap: {
            control: "boolean",
        },
        tag: {
            if: {
                arg: undefined as never,
                truthy: true,
            },
        },
    },
} satisfies Meta<typeof Button>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
    args: {
        children: "Primary",
        color: "primary",
    },
};

export const Filled: Story = {
    args: {
        children: "Filled",
    },
};

export const Outline: Story = {
    args: {
        children: "Outline",
        colorType: "outline",
    },
};

export const Text: Story = {
    args: {
        children: "Text",
        colorType: "text",
    },
};

export const Disabled: Story = {
    args: {
        children: "Disabled",
        disabled: true,
    },
};

export const Secondary: Story = {
    args: {
        children: "Secondary",
        color: "secondary",
    },
};

export const Accent: Story = {
    args: {
        children: "Accent",
        color: "accent",
    },
};

export const Neutral: Story = {
    args: {
        children: "Neutral",
        color: "neutral",
    },

};

export const NeutralContent: Story = {
    args: {
        children: "Neutral Content",
        color: "neutral-content",
    },
};

export const Info: Story = {
    args: {
        children: "Info",
        color: "info",
    },
};

export const Info700: Story = {
    args: {
        children: "Info-700",
        color: "info-700",
    },
};

export const Success: Story = {
    args: {
        children: "Success",
        color: "success",
    },
};

export const Warning: Story = {
    args: {
        children: "Warning",
        color: "warning",
    },
};

export const Error: Story = {
    args: {
        children: "Error",
        color: "error",
    },
};

