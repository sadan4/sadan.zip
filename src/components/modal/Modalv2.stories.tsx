import type { Meta, StoryObj } from "@storybook/react-vite";

import { Modal, ModalContext } from ".";
import { Button } from "../Button";
import { Box } from "../layout/Box";
import { Text } from "../Text";

import { use, useRef } from "react";


const meta = {
    args: {},
    component: Modal,
} satisfies Meta<typeof Modal>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
    args: {} as never,
    render() {
        const modal = useRef<ModalContext>(null);

        return (
            <>
                <Button onClick={() => modal.current?.open()}>
                    Open Modal
                </Button>
                <Modal ref={modal}>
                    <Modal1 />
                </Modal>
            </>
        );
    },
};

function Modal1() {
    const ctx = use(ModalContext);
    const modal = useRef<ModalContext>(null);

    return (
        <>
            <Box className="p-16">
                <Text>
                    This is a test modal
                </Text>
                <Button
                    className="mt-8"
                    onClick={() => modal.current?.open()}
                >
                    Open Second Modal
                </Button>
                <Button
                    className="mt-8"
                    onClick={ctx?.close}
                >
                    Close This Modal
                </Button>
            </Box>
            <Modal ref={modal}>
                <Modal2 />
            </Modal>
        </>
    );
}

function Modal2() {
    const ctx = use(ModalContext);
    const modal = useRef<ModalContext>(null);

    return (
        <>
            <Box className="p-16">
                <Text>
                    This is a test modal2
                </Text>
                <Button
                    className="mt-8"
                    onClick={() => modal.current?.open()}
                >
                    Open Third Modal
                </Button>
                <Button
                    className="mt-8"
                    onClick={ctx?.close}
                >
                    Close This Modal
                </Button>
            </Box>
            <Modal ref={modal}>
                <Modal3 />
            </Modal>
        </>
    );
}

function Modal3() {
    const ctx = use(ModalContext);

    return (
        <>
            <Box className="p-16">
                <Text>
                    This is a test modal3
                </Text>
                <Button
                    className="mt-8"
                    onClick={ctx?.close}
                >
                    Close This Modal
                </Button>
            </Box>
        </>
    );
}
