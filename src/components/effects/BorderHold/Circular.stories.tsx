import { Text } from "@/components/Text";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { BorderHoldCircular } from "./Circular";

import { useState } from "react";

const meta = {
    component: BorderHoldCircular,
} satisfies Meta<typeof BorderHoldCircular>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    render() {
        const [text, setText] = useState("Hold Me");

        return (
            <div className="m-16">
                <BorderHoldCircular onHold={() => {
                    setText("Held!");
                    setTimeout(() => setText("Hold Me"), 5000);
                }}
                >
                    <div className="h-16 w-16 rounded-full bg-secondary-800">
                        <Text>
                            {text}
                        </Text>
                    </div>
                </BorderHoldCircular>
            </div>
        );
    },
};
