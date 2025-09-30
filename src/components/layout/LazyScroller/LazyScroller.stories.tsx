import { Button } from "@/components/Button";
import cn from "@/utils/cn";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { LazyScroller } from ".";

import { useState } from "react";

const items = Object.freeze(Array.from({ length: 1000 }).map((_, i) => `Item ${i + 1}`));

const meta = {
    component: LazyScroller,
    args: {
        items,
        className: cn("w-min"),
        renderItem({ item }) {
            return (
                <div
                    key={item}
                    className="w-fit"
                >Entry: {item}
                </div>
            );
        },
    },
    render(args) {
        return (
            <div className="w-60">
                <LazyScroller {...args} />
            </div>
        );
    },
} satisfies Meta<typeof LazyScroller<string>>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
export const WithBuffer: Story = {
    args: {
        bufferSize: 1,
    },
};

export const HeaderAndFooter: Story = {
    args: {
        renderHeader() {
            return <div className="bg-accent-300/50 h-10 w-full text-center">Header</div>;
        },
        renderFooter() {
            return <div className="bg-secondary-500/50 h-10 w-full text-center">Footer</div>;
        },
        alwaysRenderFooter: true,
    },
};

export const StickyHeaderAndFooter: Story = {
    args: {
        renderHeader() {
            return <div className="bg-accent-300/50 sticky top-0 h-10 w-full text-center">Header</div>;
        },
        renderFooter() {
            return <div className="bg-secondary-500/50 sticky bottom-0 h-10 w-full text-center">Footer</div>;
        },
        alwaysRenderFooter: true,
    },
};

export const ChangingItems: Story = {
    args: {
    },
    render(args) {
        const [items, setItems] = useState(Array.from({ length: 50 }).map((_, i) => `Item ${i + 1}`));

        return (
            <div className="flex w-60 flex-col gap-2">
                <Button
                    colorType="outline"
                    onClick={() => setItems((items) => [...Array.from({ length: 50 }).map((_, i) => `Item ${items.length + i + 1}`), ...items])}
                >
                    Add 50 to top
                </Button>
                <Button
                    colorType="outline"
                    disabled={items.length <= 50}
                    onClick={() => setItems((items) => items.slice(50))}
                >
                    Remove 50 from top
                </Button>
                {
                    meta.render({
                        ...args,
                        className: undefined,
                        items,
                        batchSize: 50,
                    })
                }
                <Button
                    colorType="outline"
                    onClick={() => setItems((items) => [...items, ...Array.from({ length: 50 }).map((_, i) => `Item ${items.length + i + 1}`)])}
                >
                    Add 50 to bottom
                </Button>
                <Button
                    colorType="outline"
                    disabled={items.length <= 50}
                    onClick={() => setItems((items) => items.slice(0, Math.max(0, items.length - 50)))}
                >
                    Remove 50 from bottom
                </Button>
            </div>
        );
    },
};
