import type { Meta, StoryObj } from "@storybook/react-vite";

import { Tooltip } from ".";
import { Button } from "../Button";
import { Switch } from "../Switch";

import { useState } from "react";
import { TooltipPosition } from "./constants";

const meta = {
    component: Tooltip,
    args: {
        text: "Tooltip Content",
    },
    render(args) {
        return (
            <div className="flex items-center justify-center w-screen h-screen gap-10">
                <Tooltip
                    {...args}
                >
                    <Button>Hover Me</Button>
                </Tooltip>
            </div>
        );
    },
} satisfies Meta<typeof Tooltip>;

export default meta;

type Story = StoryObj<typeof Tooltip>;

export const Top: Story = {
    args: {
        position: TooltipPosition.TOP,
    },
};
export const Controlled: Story = {
    render() {
        const [show, setShow] = useState(false);

        return (
            <div className="flex items-center justify-center w-screen h-screen gap-10">
                <Tooltip
                    text="Tooltip text"
                    show={show}
                >
                    <Button>Click the switch --{">"}</Button>
                </Tooltip>
                <Switch
                    value={show}
                    onChange={setShow}
                />
            </div>
        );
    },
};
