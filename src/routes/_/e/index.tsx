import { Boilerplate } from "@/components/Boilerplate";
import { Box } from "@/components/layout/Box";
import { HorizontalLine } from "@/components/Lines";
import { TextLink } from "@/components/Links";
import { Text } from "@/components/Text";
import { Tooltip } from "@/components/Tooltip";
import { EM_DASH } from "@/utils/constants";
import { sendMessage } from "@/utils/e/socket";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";

import type { BundleInfo } from "../../../../server/types";

import { ExternalLinkIcon } from "lucide-react";


export const Route = createFileRoute("/_/e/")({
    component: RouteComponent,
    ssr: false,
});

interface BundleSelectorProps {
    bundle: BundleInfo;
}

const SEPARATOR = (
    <Text
        tag="span"
    >
        {" | "}
    </Text>
);

function BundleSelector({ bundle }: BundleSelectorProps) {
    return (
        <li className="flex justify-between">
            <div>
                <Text tag="span">
                    Build Number {EM_DASH} {bundle.buildNumber}
                </Text>
                {SEPARATOR}
                <Tooltip
                    text={bundle.buildHash}
                >
                    <Text
                        tag="span"
                        className="underline decoration-dashed underline-offset-2"
                    >
                        Build Hash
                    </Text>
                </Tooltip>
                {SEPARATOR}
                <Text tag="span">
                    First Seen {EM_DASH} {new Date(bundle.firstSeen).toLocaleString()}
                </Text>
            </div>
            <div>
                <TextLink
                    to="/e/view"
                    search={{
                        buildHash: bundle.buildHash,
                    }}
                    color="primary"
                >
                    Open Bundle <ExternalLinkIcon className="inline" />
                </TextLink>
            </div>
        </li>
    );
}

function RouteComponent() {
    const { status, data } = useQuery({
        queryKey: ["getAvailableBundles"],
        async queryFn() {
            return await sendMessage<"queryBundlesResponse">({
                type: "queryBundles",
            });
        },
    });

    const sortedBundles = data?.bundles.toSorted((a, b) => a.firstSeen - b.firstSeen);

    return (
        <>
            <Boilerplate />
            <div className="flex justify-center pt-4">
                <Box className="min-w-1/2">
                    <Text
                        size="xl"
                        center
                    >
                        Select a build
                    </Text>
                    <HorizontalLine
                        color="white-700"
                    />
                    {status === "pending" && (
                        <Text
                            size="lg"
                            color="accent"
                            center
                        >
                            Loading...
                        </Text>
                    )}
                    {status === "error" && (
                        <Text
                            size="lg"
                            color="error"
                            center
                        >
                            An error occurred while loading the bundles.
                        </Text>
                    )}
                    {status === "success" && (
                        <ul>
                            {sortedBundles!.length === 0 && (
                                <Text
                                    color="error"
                                    size="lg"
                                    center
                                >
                                    No Bundles Available.
                                    <p />
                                    This is an error. Please report this.
                                </Text>
                            )}
                            {sortedBundles!.map((bundle) => {
                                return (
                                    <BundleSelector
                                        key={bundle.buildHash}
                                        bundle={bundle}
                                    />
                                );
                            })}
                        </ul>
                    )}
                </Box>
            </div>
        </>
    );
}
