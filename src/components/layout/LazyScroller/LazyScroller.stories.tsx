import cn from "@/utils/cn";
import type { Meta, StoryObj } from "@storybook/react-vite";

import { LazyScroller } from ".";

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
