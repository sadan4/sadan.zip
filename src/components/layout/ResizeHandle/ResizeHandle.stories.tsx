import cn from "@/utils/cn";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { HorizontalResizeHandle, VerticalResizeHandle } from ".";

import { type ComponentProps, useRef } from "react";

interface ExProps extends ComponentProps<"div"> {
}

function First({ className, ...props }: ExProps) {
    return (
        <div
            className={cn("h-full w-auto bg-accent-300/50", className)}
            {...props}
        />
    );
}

function Second({ className, ...props }: ExProps) {
    return (
        <div
            className={cn("h-full w-auto grow bg-secondary-500/50", className)}
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
        const boundingElementRef = useRef<HTMLDivElement | null>(null);
        const firstRef = useRef<HTMLDivElement | null>(null);

        return (
            <div
                className="relative flex h-50 w-full"
                ref={boundingElementRef}
            >
                <First ref={firstRef} />
                <VerticalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResize={(e) => {
                        if (firstRef.current) {
                            console.log(e);
                            firstRef.current.style.width = `${e}%`;
                        }
                    }}
                />
                <Second />
            </div>
        );
    },
};

export const VerticalOnFinish: Story = {
    render() {
        const boundingElementRef = useRef<HTMLDivElement | null>(null);
        const firstRef = useRef<HTMLDivElement | null>(null);

        return (
            <div
                className="relative flex h-50 w-full"
                ref={boundingElementRef}
            >
                <First ref={firstRef} />
                <VerticalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResizeFinish={(e) => {
                        if (firstRef.current) {
                            console.log(e);
                            firstRef.current.style.width = `${e}%`;
                        }
                    }}
                />
                <Second />
            </div>
        );
    },
};
export const ManyVerticalHandles: Story = {
    render() {
        const boundingElementRef = useRef<HTMLDivElement | null>(null);
        const secondTrackRef = useRef<HTMLDivElement | null>(null);
        const firstRef = useRef<HTMLDivElement | null>(null);
        const secondRef = useRef<HTMLDivElement | null>(null);

        return (
            <div
                className="relative flex h-50 w-full"
                ref={boundingElementRef}
            >
                <div
                    className="h-full w-auto bg-accent-300/50"
                    ref={firstRef}
                />
                <VerticalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResize={(e) => {
                        if (firstRef.current) {
                            firstRef.current.style.width = `${e}%`;
                        }
                    }}
                />
                <div
                    className="relative flex h-full grow"
                    ref={secondTrackRef}
                >
                    <div
                        className="h-full bg-secondary-500/50"
                        ref={secondRef}
                    />
                    <VerticalResizeHandle
                        boundingElementRef={secondTrackRef}
                        onResize={(e) => {
                            if (secondRef.current) {
                                secondRef.current.style.width = `${e}%`;
                            }
                        }}
                    />
                    <div className="h-full grow bg-warning-300/50" />
                </div>
            </div>
        );
    },
};

export const Horizontal: Story = {
    render() {
        const boundingElementRef = useRef<HTMLDivElement | null>(null);
        const firstRef = useRef<HTMLDivElement | null>(null);

        return (
            <div
                className="relative flex h-120 w-40 flex-col"
                ref={boundingElementRef}
            >
                <div
                    ref={firstRef}
                    className="w-full bg-accent-300/50"
                />
                <HorizontalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResize={(e) => {
                        if (firstRef.current) {
                            firstRef.current.style.height = `${e}%`;
                        }
                    }}
                />
                <div className="w-full grow bg-secondary-500/50" />
            </div>
        );
    },
};

export const ManyHorizontalHandles: Story = {
    render() {
        const boundingElementRef = useRef<HTMLDivElement | null>(null);
        const secondTrackRef = useRef<HTMLDivElement | null>(null);
        const firstRef = useRef<HTMLDivElement | null>(null);
        const secondRef = useRef<HTMLDivElement | null>(null);

        return (
            <div
                className="relative flex h-120 w-40 flex-col"
                ref={boundingElementRef}
            >
                <div
                    className="w-full bg-accent-300/50"
                    ref={firstRef}
                />
                <HorizontalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResize={(e) => {
                        if (firstRef.current) {
                            firstRef.current.style.height = `${e}%`;
                        }
                    }}
                />
                <div
                    className="relative flex w-full grow flex-col"
                    ref={secondTrackRef}
                >
                    <div
                        className="w-full bg-secondary-500/50"
                        ref={secondRef}
                    />
                    <HorizontalResizeHandle
                        boundingElementRef={secondTrackRef}
                        onResize={(e) => {
                            if (secondRef.current) {
                                secondRef.current.style.height = `${e}%`;
                            }
                        }}
                    />
                    <div className="w-full grow bg-warning-300/50" />
                </div>
            </div>
        );
    },
};

export const HorizontalOnFinish: Story = {
    render() {
        const boundingElementRef = useRef<HTMLDivElement | null>(null);
        const firstRef = useRef<HTMLDivElement | null>(null);

        return (
            <div
                className="relative flex h-120 w-40 flex-col"
                ref={boundingElementRef}
            >
                <div
                    ref={firstRef}
                    className="w-full bg-accent-300/50"
                />
                <HorizontalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResizeFinish={(e) => {
                        if (firstRef.current) {
                            firstRef.current.style.height = `${e}%`;
                        }
                    }}
                />
                <div className="w-full grow bg-secondary-500/50" />
            </div>
        );
    },
};
