import { Slider } from "@/components/Slider";
import { Text } from "@/components/Text";
import { makeRange } from "@/utils/array";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { BorderHold } from ".";

import { useRef, useState } from "react";

const meta = {
    component: BorderHold,
} satisfies Meta<typeof BorderHold>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    render() {
        const [text, setText] = useState("Hold Me");
        const [borderRadius, setBorderRadius] = useState(24);
        const handle = useRef<BorderHold.Handle>(null);

        return (
            <div className="m-16 flex flex-col justify-items-center gap-4">
                <BorderHold
                    onHold={() => {
                        setText("Held!");
                        setTimeout(() => setText("Hold Me"), 5000);
                    }}
                    ref={handle}
                >
                    <div
                        className="flex h-16 w-16 items-center justify-center bg-secondary-800"
                        style={{
                            borderRadius,
                        }}
                    >
                        <Text noselect>
                            {text}
                        </Text>
                    </div>
                </BorderHold>
                <Slider
                    min={0}
                    max={32}
                    onChange={(value) => {
                        setBorderRadius(value);
                        handle.current?.recalculateBorder();
                    }}
                    markers={makeRange(0, 32, 8)}
                    vertical
                />
            </div>
        );
    },
};
