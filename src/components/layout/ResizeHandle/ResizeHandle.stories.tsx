import cn from "@/utils/cn";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { HorizontalResizeHandle, VerticalResizeHandle } from ".";

import { type ComponentProps, useRef } from "react";

interface ExProps extends ComponentProps<"div"> {
}

function First({ className, ...props }: ExProps) {
    return (
        <div
            className={cn("bg-accent-300/50 h-full w-auto", className)}
            {...props}
        />
    );
}

function Second({ className, ...props }: ExProps) {
    return (
        <div
            className={cn("bg-secondary-500/50 h-full w-auto grow", className)}
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
                className="flex w-full h-50 relative"
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
                className="flex w-full h-50 relative"
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
                className="flex w-full h-50 relative"
                ref={boundingElementRef}
            >
                <div
                    className="bg-accent-300/50 h-full w-auto"
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
                    className="grow h-full flex relative"
                    ref={secondTrackRef}
                >
                    <div
                        className="bg-secondary-500/50 h-full"
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
                    <div className="bg-warning-300/50 h-full grow" />
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
                className="flex flex-col relative w-40 h-120"
                ref={boundingElementRef}
            >
                <div
                    ref={firstRef}
                    className="bg-accent-300/50 w-full"
                />
                <HorizontalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResize={(e) => {
                        if (firstRef.current) {
                            firstRef.current.style.height = `${e}%`;
                        }
                    }}
                />
                <div className="bg-secondary-500/50 w-full grow" />
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
                className="flex flex-col w-40 h-120 relative"
                ref={boundingElementRef}
            >
                <div
                    className="bg-accent-300/50 w-full"
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
                    className="grow w-full flex flex-col relative"
                    ref={secondTrackRef}
                >
                    <div
                        className="bg-secondary-500/50 w-full"
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
                    <div className="bg-warning-300/50 w-full grow" />
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
                className="flex flex-col relative w-40 h-120"
                ref={boundingElementRef}
            >
                <div
                    ref={firstRef}
                    className="bg-accent-300/50 w-full"
                />
                <HorizontalResizeHandle
                    boundingElementRef={boundingElementRef}
                    onResizeFinish={(e) => {
                        if (firstRef.current) {
                            firstRef.current.style.height = `${e}%`;
                        }
                    }}
                />
                <div className="bg-secondary-500/50 w-full grow" />
            </div>
        );
    },
};
