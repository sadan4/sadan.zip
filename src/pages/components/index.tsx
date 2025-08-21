import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/Box";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { TabBar } from "@/components/TabBar";
import { Text } from "@/components/Text";

export default function Components() {
    return (
        <>
            <Boilerplate />
            <FooterContainer
                footer={() => <DefaultFooter />}
            >
                <div className="mt-4 flex flex-col items-center justify-center">
                    <Text
                        size="4xl"
                        weight="extraBold"
                    >
                        Component Testing
                    </Text>
                    <Box className="mt-6 w-[40vw]">
                        <Text size="xl">Tab Bar</Text>
                        <TabBar tabs={[
                            {
                                id: "1",
                                render() {
                                    return <Text size="md">Tab 1 content</Text>;
                                },
                                renderTab() {
                                    return (
                                        <Text
                                            size="md"
                                            noselect
                                        >Tab 1
                                        </Text>
                                    );
                                },
                            },
                            {
                                id: "2",
                                render() {
                                    return <Text size="md">Tab 2 content</Text>;
                                },
                                renderTab() {
                                    return (
                                        <Text
                                            size="md"
                                            noselect
                                        >Tab 2
                                        </Text>
                                    );
                                },
                            },
                            {
                                id: "3",
                                render() {
                                    return <Text size="md">Tab 3 content</Text>;
                                },
                                renderTab() {
                                    return (
                                        <Text
                                            size="md"
                                            noselect
                                        >Tab 3
                                        </Text>
                                    );
                                },
                            },
                        ]}
                        />
                    </Box>
                </div>
            </FooterContainer>
        </>
    );
}
