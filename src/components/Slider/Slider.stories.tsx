import { rangeInputDefaultValue } from "@/utils/dom";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { Slider } from ".";
import { Button } from "../Button";
import { Input } from "../Input";
import { Text } from "../Text";

import { useEffect, useState } from "react";


const meta = {
    component: Slider,
    render({ value, onChange, initialValue, min, max, ...props }) {
        const [cur, setCur] = useState(value ?? initialValue ?? (((min ?? 0) + (max ?? 100)) / 2));

        useEffect(() => {
            if (value != null) {
                setCur(value);
            }
        }, [value]);
        return (
            <div>
                <Slider
                    value={value}
                    initialValue={initialValue}
                    min={min}
                    max={max}
                    onChange={(next) => {
                        if (value == null) {
                            setCur(next);
                        }
                        onChange?.(next);
                    }}
                    {...props}
                />
                <Text>{cur}</Text>
            </div>
        );
    },
} satisfies Meta<typeof Slider>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
    },
};

export const Disabled: Story = {
    args: {
        disabled: true,
    },
};

export const WithMarkers: Story = {
    args: {
        markers: Array.from({ length: 11 }, (_, i) => i * 10),
    },
};

export const SnapToMarkers: Story = {
    args: {
        markers: Array.from({ length: 11 }, (_, i) => i * 10),
        stickToMarkers: "snap",
    },
};

export const StickToMarkers: Story = {
    args: {
        markers: Array.from({ length: 11 }, (_, i) => i * 10),
        stickToMarkers: "stick",
    },
};

export const StickToMarkersWithoutShowingThem: Story = {
    args: {
        markers: Array.from({ length: 11 }, (_, i) => i * 10),
        stickToMarkers: "stick",
        showMarkers: false,
    },
};

export const WeaklyControlled: Story = {
    args: {

    },
    render(props) {
        const [value, setValue] = useState(rangeInputDefaultValue(props.min, props.max));

        return (
            <div>
                <Slider
                    {...props}
                    value={value}
                    onChange={(newValue) => setValue(newValue)}
                />
                <Text>{value}</Text>
            </div>
        );
    },
};

export const Controlled: Story = {
    args: {

    },
    render(props) {
        const [value, setValue] = useState(rangeInputDefaultValue(props.min, props.max));
        const [content, setContent] = useState<number | null>(null);

        return (
            <div>
                <Slider
                    {...props}
                    value={value}
                    onChange={(newValue) => setValue(newValue)}
                />
                <Text>{value}</Text>
                <div className="flex gap-3">
                    <Input
                        placeholder="Enter a value"
                        onChange={(e) => {
                            const str = e.target.value;
                            const v = +str;

                            if (str !== "" && !Number.isNaN(v)) {
                                setContent(v);
                            } else {
                                setContent(null);
                            }
                        }}
                    />
                    <Button
                        disabled={content == null}
                        onClick={() => {
                            if (content != null) {
                                setValue(content);
                            }
                        }}
                    >
                        Update Slider
                    </Button>
                </div>
            </div>
        );
    },
};

export const ExtraSmall: Story = {
    args: {
        size: "xs",
    },
};

export const Small: Story = {
    args: {
        size: "sm",
    },
};

export const Medium: Story = {
    args: {
        size: "md",
    },
};

export const Large: Story = {
    args: {
        size: "lg",
    },
};
