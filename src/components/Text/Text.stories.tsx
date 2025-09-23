import type { Meta, StoryObj } from "@storybook/react-vite";

import { Text } from ".";

const meta = {
    component: Text,
} satisfies Meta<typeof Text>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        children: "Hello World",
    },
};

export const NoSelect: Story = {
    args: {
        children: "No Select",
        noselect: true,
    },
};

export const Center: Story = {
    args: {
        children: "Centered!",
        center: true,
    },
};

export const Primary: Story = {
    args: {
        children: "Hello World",
        color: "primary",
    },
};

export const Secondary: Story = {
    args: {
        children: "Hello World",
        color: "secondary",
    },
};

export const Accent: Story = {
    args: {
        children: "Hello World",
        color: "accent",
    },
};

export const Neutral: Story = {
    args: {
        children: "Hello World",
        color: "neutral",
    },
};

export const NeutralContent: Story = {
    args: {
        children: "Hello World",
        color: "neutral-content",
    },
};

export const Info: Story = {
    args: {
        children: "Hello World",
        color: "info",
    },
};

export const Info600: Story = {
    args: {
        children: "Hello World",
        color: "info-600",
    },
};

export const Info700: Story = {
    args: {
        children: "Hello World",
        color: "info-700",
    },
};

export const Success: Story = {
    args: {
        children: "Hello World",
        color: "success",
    },
};

export const Warning: Story = {
    args: {
        children: "Hello World",
        color: "warning",
    },
};

export const Error: Story = {
    args: {
        children: "Hello World",
        color: "error",
    },
};

export const Black: Story = {
    args: {
        children: "Hello World",
        color: "black",
    },
};

export const Black200: Story = {
    args: {
        children: "Hello World",
        color: "black-200",
    },
};

export const Black300: Story = {
    args: {
        children: "Hello World",
        color: "black-300",
    },
};

export const White: Story = {
    args: {
        children: "Hello World",
        color: "white",
    },
};

export const White600: Story = {
    args: {
        children: "Hello World",
        color: "white-600",
    },
};

export const White700: Story = {
    args: {
        children: "Hello World",
        color: "white-700",
    },
};

export const White800: Story = {
    args: {
        children: "Hello World",
        color: "white-800",
    },
};

export const Xs: Story = {
    args: {
        children: "Hello World (xs)",
        size: "xs",
    },
};

export const Sm: Story = {
    args: {
        children: "Hello World (sm)",
        size: "sm",
    },
};

export const Md: Story = {
    args: {
        children: "Hello World (md)",
        size: "md",
    },
};

export const Lg: Story = {
    args: {
        children: "Hello World (lg)",
        size: "lg",
    },
};

export const Xl: Story = {
    args: {
        children: "Hello World (xl)",
        size: "xl",
    },
};

export const Xl2: Story = {
    args: {
        children: "Hello World (2xl)",
        size: "2xl",
    },
};

export const Xl3: Story = {
    args: {
        children: "Hello World (3xl)",
        size: "3xl",
    },
};

export const Xl4: Story = {
    args: {
        children: "Hello World (4xl)",
        size: "4xl",
    },
};

export const Xl5: Story = {
    args: {
        children: "Hello World (5xl)",
        size: "5xl",
    },
};

export const Xl6: Story = {
    args: {
        children: "Hello World (6xl)",
        size: "6xl",
    },
};

export const Xl7: Story = {
    args: {
        children: "Hello World (7xl)",
        size: "7xl",
    },
};

export const Xl8: Story = {
    args: {
        children: "Hello World (8xl)",
        size: "8xl",
    },
};

export const Xl9: Story = {
    args: {
        children: "Hello World (9xl)",
        size: "9xl",
    },
};

