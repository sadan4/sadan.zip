import type { Meta, StoryObj } from "@storybook/react-vite";

import sampleTsx from "./Codeblock.tsx?raw";
import { Codeblock, CodeblockLang } from ".";

const meta = {
    component: Codeblock,
    render(props) {
        const C = this.component!;

        return (
            <div className="max-w-3/4">
                <C {...props} />
            </div>
        );
    },
} satisfies Meta<typeof Codeblock>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {
        lang: CodeblockLang.TSX,
        children: sampleTsx,
    },
};
