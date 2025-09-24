import type { Meta, StoryObj } from "@storybook/react-vite";

import { Tooltip } from ".";
import { Button } from "../Button";
import { Switch } from "../Switch";
import { useState } from "react";

const meta = {
    component: Tooltip,
} satisfies Meta<typeof Tooltip>;

export default meta;

type Story = StoryObj<typeof Tooltip>;

export const Default: Story = {
    render() {
        const [show, setShow] = useState(false);

        return (
            <div className="flex items-center p-10 gap-10">
                <Tooltip
                    text="Tooltip text"
                    show={show}
                >
                    <Button>Hover Me</Button>
                </Tooltip>
                <Tooltip
                    text="Tooltip text"
                >
                    <Button>Hover Me</Button>
                </Tooltip>
                <Switch
                    value={show}
                    onChange={setShow}
                />
            </div>
        );
    },
};
