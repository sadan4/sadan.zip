import { Text } from "@/components/Text";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { BorderHoldRectangular } from "./Rectangular";

import { useState } from "react";

const meta = {
    component: BorderHoldRectangular,
} satisfies Meta<typeof BorderHoldRectangular>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    render() {
        const [text, setText] = useState("Hold Me");

        return (
            <div className="m-16">
                <BorderHoldRectangular onHold={() => {
                    setText("Held!");
                    setTimeout(() => setText("Hold Me"), 5000);
                }}
                >
                    <div className="h-16 w-16 bg-secondary-800">
                        <Text>
                            {text}
                        </Text>
                    </div>
                </BorderHoldRectangular>
            </div>
        );
    },
};
