import { Boilerplate } from "@/components/Boilerplate.tsrx";
import { Box } from "@/components/layout/Box";
import { HorizontalLine } from "@/components/Lines";
import { TextLink } from "@/components/Links";
import { Text } from "@/components/Text";
import { Tooltip } from "@/components/Tooltip";
import { EM_DASH } from "@/utils/constants";
import type { TBundleHash } from "@/utils/types";
import { get_builds, Meta } from "@sadan4/libsadancore";
import { useQuery } from "@tanstack/react-query";

import { ExternalLinkIcon } from "lucide-react";
import { useMemo } from "react";

interface BundleItemProps {
    bundleMeta: Meta;
}

const SEPARATOR = (
    <Text
        tag="span"
    >
        {" | "}
    </Text>
);

function BundleItem({ bundleMeta }: BundleItemProps) {
    return (
        <li className="flex justify-between">
            <div>
                <Text tag="span">
                    Build Number {EM_DASH} {bundleMeta.build_number}
                </Text>
                {SEPARATOR}
                <Tooltip
                    text={bundleMeta.build_hash}
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
                    First Seen {EM_DASH} {new Date(Number(bundleMeta.first_seen)).toLocaleString()}
                </Text>
            </div>
            <div>
                <TextLink
                    to="/e/view/{-$buildHash}/{-$moduleId}"
                    params={{
                        buildHash: bundleMeta.build_hash as TBundleHash,
                        moduleId: null,
                    }}
                    color="primary"
                >
                    Open Bundle <ExternalLinkIcon className="inline" />
                </TextLink>
            </div>
        </li>
    );
}

export function BundleSelector() {
    const { status, data } = useQuery({
        queryKey: ["getAvailableBundles"],
        queryFn() {
            return get_builds();
        },
    });

    const sortedBundles = useMemo(() => data?.toSorted(Meta.sort_newest_first), [data]);

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
                            {sortedBundles!.map((bundleMeta) => {
                                return (
                                    <BundleItem
                                        key={bundleMeta.build_hash}
                                        bundleMeta={bundleMeta}
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
