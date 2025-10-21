import type { Meta, StoryObj } from "@storybook/react-vite";

import * as api from "./index";
import { Button } from "../Button";
import { Box } from "../layout/Box";
import { Text } from "../Text";

function openModal3() {
    api.openModal({
        key: api.ModalKey.TESTING3,
        Render() {
            return (
                <Box className="p-16">
                    <Text>
                        This is a third test modal foo
                    </Text>
                    <Button
                        onClick={this.close}
                        className="mt-8"
                    >
                        Close This Modal
                    </Button>
                </Box>
            );
        },
    });
}

function openModal2() {
    api.openModal({
        key: api.ModalKey.TESTING2,
        Render() {
            return (
                <Box className="p-16">
                    <Text>
                        This is a second test modal
                    </Text>
                    <Button
                        className="mt-8"
                        onClick={openModal3}
                    >
                        Open Third Modal
                    </Button>
                    <Button
                        onClick={this.close}
                        className="mt-8"
                    >
                        Close This Modal
                    </Button>
                </Box>
            );
        },
    });
}

function openModal() {
    api.openModal({
        key: api.ModalKey.TESTING,
        Render() {
            return (
                <Box className="p-16">
                    <Text>
                        This is a test modal
                    </Text>
                    <Button
                        className="mt-8"
                        onClick={openModal2}
                    >
                        Open Second Modal
                    </Button>
                    <Button
                        onClick={this.close}
                        className="mt-8"
                    >
                        Close This Modal
                    </Button>
                </Box>
            );
        },
    });
}

const meta = {
    title: "Components/Modal",
    render() {
        return (
            <Button onClick={openModal}>
                Open Modal
            </Button>
        );
    },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
};
