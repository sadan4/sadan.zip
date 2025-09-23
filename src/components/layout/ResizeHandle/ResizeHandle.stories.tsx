import cn from "@/utils/cn";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { HorizontalResizeHandle, VerticalResizeHandle } from ".";

import type { ComponentProps } from "react";

interface ExProps extends ComponentProps<"div"> {
}

function First({ className, ...props }: ExProps) {
    return (
        <div
            className={cn("bg-bg-300 h-full w-full inset-ring-2 inset-ring-secondary", className)}
            {...props}
        />
    );
}

function Second({ className, ...props }: ExProps) {
    return (
        <div
            className={cn("bg-bg-300 h-full w-full inset-ring-2 inset-ring-accent", className)}
            {...props}
        />
    );
}

const meta = {
    title: "Components/Layout/ResizeHandle",
    subcomponents: {
        VerticalResizeHandle,
        HorizontalResizeHandle,
    },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Vertical: Story = {
    render() {
        return (
            <div className="flex w-full h-50">
                <First />
                <VerticalResizeHandle />
                <Second />
            </div>
        );
    },
};
