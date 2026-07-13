import { Clickable } from "@/components/Clickable";
import { CheckedInput, type LenCheck } from "@/components/Input";
import { validateLength } from "@/components/Input/util";
import { BufferedScroller } from "@/components/layout/BufferedScroller";
import { LabeledSwitch } from "@/components/Switch/index";
import { Text } from "@/components/Text";
import { makeExclusiveRange } from "@/utils/array";
import type { TModuleId } from "@/utils/types";
import { useQuery } from "@tanstack/react-query";

import { useModuleViewerSettingsStore, useModuleViewerStore } from "../../-data";
import type { RemoteBuildService } from "../../-data/worker/api";
import type { BundleSearchResults } from "../../-data/worker/sharedWorker";
import { Route } from "../../view.{-$buildHash}.{-$moduleId}";

import "@xyflow/react/dist/style.css";
import { useState } from "react";


const EMPTY_SEARCH_RESULTS = {
    moduleIds: new Uint32Array(),
    rawIndices: new Uint32Array(),
} satisfies BundleSearchResults;

// function handleChangeLongSearchPreviews(longSearchPreviews: boolean) {
//     ModuleViewerSettingsStore.setState({ longSearchPreviews });
// }

const SEARCH_CHECK = Object.freeze({
    type: "len",
    min: 3,
} satisfies LenCheck);

// TODO: validate regex clientside
export function ModuleSearch() {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const longSearchPreviews = useModuleViewerSettingsStore(({ longSearchPreviews }) => longSearchPreviews);
    // the query that is being searched for.
    const [query, setQuery] = useState("");
    const [regexSearch, setRegexSearch] = useState(false);

    const { data = EMPTY_SEARCH_RESULTS, status, error } = useQuery({
        queryKey: ["ExplorerSidebarSearch", buildHash, query, regexSearch],
        async queryFn(): Promise<BundleSearchResults> {
            if (!query) {
                return EMPTY_SEARCH_RESULTS;
            }

            return await buildService.searchModules(query, regexSearch);
        },
    });

    const statusEl = function () {
        switch (status) {
            case "pending": {
                return (
                    <Text>
                        Searching...
                    </Text>
                );
            }
            case "error": {
                return (
                    <Text color="error">
                        An error occurred while searching for modules:
                        {" "}
                        {error instanceof Error ? error.message : String(error)}
                    </Text>
                );
            }
            case "success": {
                if (data === EMPTY_SEARCH_RESULTS) {
                    return (
                        <Text>
                            Enter a query above
                        </Text>
                    );
                }
                return (
                    <Text color="success">
                        {data.moduleIds.length} results found
                    </Text>
                );
            }
        }
    }();

    return (
        <div className="flex w-full shrink-0 flex-col gap-2 px-2">
            <CheckedInput
                placeholder="Search Modules"
                className="mt-2 w-full"
                check={(e) => {
                    if (!e) {
                        return true;
                    }
                    return validateLength(SEARCH_CHECK, e);
                }}
                debounce={300}
                onValidChange={(e) => {
                    if (e) {
                        setQuery(e.target.value);
                    }
                }}
                onClear={() => {
                    setQuery("");
                }}
            />
            <LabeledSwitch
                value={regexSearch}
                onChange={setRegexSearch}
            >
                Regex Search
            </LabeledSwitch>
            {/* <LabeledSwitch
                value={longSearchPreviews}
                onChange={handleChangeLongSearchPreviews}
            >
                Long Previews
            </LabeledSwitch> */}
            {statusEl}
            <div>
                {status === "success" && data.moduleIds.length > 0 && (
                    <ResultsList
                        results={data}
                        longPreview={longSearchPreviews}
                        buildService={buildService}
                        buildHash={buildHash}
                    />
                )}
            </div>
        </div>
    );
}

interface ItemProps extends ResultsListProps {
    idx: number;
}

function Item({ results, idx, longPreview, buildService, buildHash }: ItemProps) {
    const navigate = Route.useNavigate();
    const moduleId = results.moduleIds[idx] as TModuleId;

    const { data } = useQuery({
        queryKey: [
            "ExplorerSideBarSearchResultsItem",
            buildHash,
            moduleId,
            longPreview,
            // we need to compare the array by shallow value
            results.rawIndices.length,
            idx,
        ],
        queryFn() {
            return buildService.getSearchResultInfo(moduleId, results.rawIndices[idx], longPreview);
        },
        retry: false,
        staleTime: Infinity,
    });

    return (
        <Clickable
            onClick={async () => {
                const location = await buildService.getSearchLocation(moduleId, results.rawIndices[idx]);

                navigate({
                    to: "/e/view/{-$buildHash}/{-$moduleId}",
                    params: {
                        moduleId,
                    },
                    search: {
                        sl: location.lineNumber,
                        sc: location.column,
                    },
                });
            }}
            className="mb-1 border-b border-fg-700"
        >
            <Text
                size="xs"
                weight="bold"
                color="accent"
            >
                {moduleId}
                .js:
                {data?.lineNumber ?? "?"}
            </Text>
            <Text size="sm">
                {data?.preview ?? "Loading Preview..."}
            </Text>
        </Clickable>
    );
}

interface ResultsListProps {
    results: BundleSearchResults;
    longPreview: boolean;
    buildService: RemoteBuildService;
    buildHash: string;
}

function ResultsList({ results, longPreview, buildService, buildHash }: ResultsListProps) {
    const items = makeExclusiveRange(0, results.moduleIds.length);

    return (
        <BufferedScroller
            items={items}
            batchSize={20}
            bufferSize={2}
            renderItem={({ item }) => {
                return (
                    <Item
                        // keyed by index is ok here because we never shrink the array
                        key={item}
                        idx={item}
                        results={results}
                        longPreview={longPreview}
                        buildService={buildService}
                        buildHash={buildHash}
                    />
                );
            }}
        />
    );
}

