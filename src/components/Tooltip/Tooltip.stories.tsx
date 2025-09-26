import type { Meta, StoryObj } from "@storybook/react-vite";

import { TooltipPosition } from "./constants";
import { Tooltip } from ".";
import { Button } from "../Button";
import { Switch } from "../Switch";

import { useState } from "react";

const meta = {
    component: Tooltip,
    args: {
        text: "Tooltip Content",
        children: <Button>Hover Me</Button>,
    },
    render(args) {
        return (
            <div className="flex h-screen w-screen items-center justify-center gap-10">
                <Tooltip
                    {...args}
                />
            </div>
        );
    },
} satisfies Meta<typeof Tooltip>;

document.body.style.padding = "0";

export default meta;

type Story = StoryObj<typeof Tooltip>;

export const Top: Story = {
    args: {
        position: TooltipPosition.TOP,
    },
};

export const Bottom: Story = {
    args: {
        position: TooltipPosition.BOTTOM,
    },
};

export const Left: Story = {
    args: {
        position: TooltipPosition.LEFT,
    },
};

export const Right: Story = {
    args: {
        position: TooltipPosition.RIGHT,
    },
};

export const Delayed: Story = {
    args: {
        hoverShowDelay: 1000,
        children: <Button>1s hover delay</Button>,
    },
};

export const Controlled: Story = {
    args: {
        text: "Controlled Tooltip",
        children: <Button>Click the switch --{">"}</Button>,
    },
    render(args) {
        const [show, setShow] = useState(false);

        return (
            <div className="flex h-screen w-screen items-center justify-center gap-10">
                <Tooltip
                    {...args}
                    show={show}
                />
                <Switch
                    value={show}
                    onChange={setShow}
                />
            </div>
        );
    },
};

export const WideTooltip: Story = {
    args: {
        text: Array
            .from({ length: 3 }, () => "This is a very wide tooltip")
            .join(" "),
        children: <Button>button</Button>,
    },
};


export const CustomContent: Story = {
    args: {
        noWrapper: true,
    },
};
