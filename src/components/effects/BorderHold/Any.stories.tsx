import { Text } from "@/components/Text";
import type { Meta, StoryObj } from "@storybook/react-vite";

import Any from "./Any";

import { useState } from "react";

const meta = {
    component: Any,
} satisfies Meta<typeof Any>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    render() {
        const [text, setText] = useState("Hold Me");

        return (
            <div className="m-16">
                <Any onHold={() => {
                    setText("Held!");
                    setTimeout(() => setText("Hold Me"), 5000);
                }}
                >
                    <div className="bg-secondary-800 h-16 w-16 rounded-full">
                        <Text>
                            {text}
                        </Text>
                    </div>
                </Any>
            </div>
        );
    },
};
