import { Button, IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { Input } from "@/components/Input";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { Text } from "@/components/Text";
import { TooltipPosition } from "@/components/Tooltip/constants";
import type { TBundleHash, TModuleId } from "@/utils/types";
import type { ExperimentInfo, ExperimentScope, ExperimentVariant } from "@sadan4/libsadancore";
import { Link } from "@tanstack/react-router";

import { Badge, ConfigBlock, FilterPills } from "./experimentComponents";
import {
    countBy,
    EXPERIMENT_SCOPES,
    EXPERIMENT_TYPES,
    experimentScopeLabels,
    type ExperimentType,
    experimentTypeLabels,
    sortByDate,
    toggledSet,
} from "./experimentUtils";
import { ModuleViewerStore, ViewMode } from "../-data";
import {
    type ChangedExperiment,
    diffExperiments,
    type DiffStatus,
    type ExperimentDiffEntry,
    latestOf,
    type ValueChange,
} from "../-data/experimentDiff";
import { getBuildService } from "../-data/worker/api";
import { Route } from "../diff.$hashA.$hashB";

import {
    ArrowLeftRightIcon,
    ArrowRightIcon,
    ChevronDownIcon,
    ChevronRightIcon,
    SquareArrowOutUpRightIcon,
} from "lucide-react";
import { useDeferredValue, useMemo, useState } from "react";

const DIFF_STATUSES: readonly DiffStatus[] = ["added", "removed", "changed"];

const diffStatusLabels: Record<DiffStatus, string> = {
    added: "Added",
    removed: "Removed",
    changed: "Changed",
};

const diffStatusColors = {
    added: "success",
    removed: "error",
    changed: "warning",
} as const satisfies Record<DiffStatus, string>;

function variantTitle({ key, label }: ExperimentVariant): string {
    return label != null ? `${key}: ${label}` : key;
}

function formatValue(value: string | null): string {
    return value ?? "none";
}

interface ValueChangeLineProps {
    title: string;
    change: ValueChange<string | null> | undefined;
}

function ValueChangeLine({ title, change }: ValueChangeLineProps) {
    if (!change) {
        return null;
    }

    return (
        <div className="flex flex-wrap items-center gap-2">
            <Text
                size="sm"
                weight="bold"
            >
                {title}:
            </Text>
            <Text
                size="sm"
                color="error"
                className="line-through"
            >
                {formatValue(change.before)}
            </Text>
            <ArrowRightIcon className="size-4 shrink-0" />
            <Text
                size="sm"
                color="success"
            >
                {formatValue(change.after)}
            </Text>
        </div>
    );
}

interface BeforeAfterProps {
    title: string;
    before: unknown;
    after: unknown;
}

function BeforeAfter({ title, before, after }: BeforeAfterProps) {
    return (
        <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
            <ConfigBlock
                title={`${title} (before)`}
                config={before}
            />
            <ConfigBlock
                title={`${title} (after)`}
                config={after}
            />
        </div>
    );
}

interface ExperimentConfigsProps {
    experiment: ExperimentInfo;
}

function ExperimentConfigs({ experiment: { defaultConfig, variants } }: ExperimentConfigsProps) {
    return (
        <>
            <ConfigBlock
                title="Default Config"
                config={defaultConfig}
            />
            {variants.map((variant) => (
                <ConfigBlock
                    key={variant.key}
                    title={variantTitle(variant)}
                    config={variant.config}
                />
            ))}
        </>
    );
}

interface ExperimentChangesViewProps {
    entry: ChangedExperiment;
}

function ExperimentChangesView({ entry: { changes } }: ExperimentChangesViewProps) {
    const { variants } = changes;

    return (
        <>
            <ValueChangeLine
                title="Type"
                change={changes.type}
            />
            <ValueChangeLine
                title="Scope"
                change={changes.scope}
            />
            <ValueChangeLine
                title="Label"
                change={changes.label}
            />
            {changes.defaultConfig && (
                <BeforeAfter
                    title="Default Config"
                    before={changes.defaultConfig.before}
                    after={changes.defaultConfig.after}
                />
            )}
            {variants.added.map((variant) => (
                <ConfigBlock
                    key={`added-${variant.key}`}
                    title={`Added variant ${variantTitle(variant)}`}
                    config={variant.config}
                />
            ))}
            {variants.removed.map((variant) => (
                <ConfigBlock
                    key={`removed-${variant.key}`}
                    title={`Removed variant ${variantTitle(variant)}`}
                    config={variant.config}
                />
            ))}
            {variants.changed.map(({ key, before, after }) => (
                <div
                    key={`changed-${key}`}
                    className="flex flex-col gap-2"
                >
                    <ValueChangeLine
                        title={`Variant ${key} label`}
                        change={before.label === after.label
                            ? undefined
                            : {
                                before: before.label,
                                after: after.label,
                            }}
                    />
                    <BeforeAfter
                        title={`Variant ${variantTitle(after)}`}
                        before={before.config}
                        after={after.config}
                    />
                </div>
            ))}
        </>
    );
}

interface GoToExperimentButtonProps {
    buildHash: TBundleHash;
    experiment: ExperimentInfo;
    /**
     * shown next to the icon, to tell the before and after links apart
     */
    sideLabel?: string;
}

function GoToExperimentButton({ buildHash, experiment: { moduleId, rawIndex }, sideLabel }: GoToExperimentButtonProps) {
    const navigate = Route.useNavigate();
    const label = `Go to ${moduleId}.js in build ${buildHash.slice(0, 9)}`;

    async function goTo() {
        const buildService = await getBuildService(buildHash);
        const location = await buildService.getSearchLocation(moduleId as TModuleId, rawIndex);

        await navigate({
            to: "/e/view/{-$buildHash}/{-$moduleId}",
            params: {
                buildHash,
                moduleId,
            },
            search: {
                sl: location.lineNumber,
                sc: location.column,
            },
        });
        ModuleViewerStore.getState().updateActivePanel(ViewMode.CODE);
    }

    if (sideLabel == null) {
        return (
            <IconButton
                label={label}
                colorType="text"
                tooltipPosition={TooltipPosition.LEFT}
                onClick={async () => {
                    await goTo();
                    return null;
                }}
            >
                <SquareArrowOutUpRightIcon />
            </IconButton>
        );
    }

    return (
        <Button
            size="sm"
            colorType="text"
            title={label}
            className="shrink-0"
            onClick={goTo}
        >
            <span className="flex items-center gap-1">
                {sideLabel}
                <SquareArrowOutUpRightIcon className="size-4" />
            </span>
        </Button>
    );
}

interface DiffRowProps {
    entry: ExperimentDiffEntry;
}

function DiffRow({ entry }: DiffRowProps) {
    const { hashA, hashB } = Route.useParams();
    const [open, setOpen] = useState(false);
    const { name, status } = entry;
    const { type, scope, label } = latestOf(entry);

    return (
        <>
            <div className="flex items-center gap-2 p-2">
                <Clickable
                    className="flex min-w-0 grow items-center gap-2"
                    onClick={() => {
                        setOpen((o) => !o);
                    }}
                >
                    {open ? <ChevronDownIcon className="shrink-0" /> : <ChevronRightIcon className="shrink-0" />}
                    <div className="flex min-w-0 flex-col">
                        <div className="flex flex-wrap items-center gap-2">
                            <Text weight="bold">
                                {name}
                            </Text>
                            <Badge textColor={diffStatusColors[status]}>{status}</Badge>
                            <Badge textColor="accent">{type}</Badge>
                            <Badge textColor="secondary">{scope}</Badge>
                        </div>
                        {label != null && label !== name && (
                            <Text
                                size="sm"
                                color="white-600"
                            >
                                {label}
                            </Text>
                        )}
                    </div>
                </Clickable>
                {entry.status === "changed" && (
                    <>
                        <GoToExperimentButton
                            buildHash={hashA}
                            experiment={entry.before}
                            sideLabel="Before"
                        />
                        <GoToExperimentButton
                            buildHash={hashB}
                            experiment={entry.after}
                            sideLabel="After"
                        />
                    </>
                )}
                {entry.status === "added" && (
                    <GoToExperimentButton
                        buildHash={hashB}
                        experiment={entry.after}
                    />
                )}
                {entry.status === "removed" && (
                    <GoToExperimentButton
                        buildHash={hashA}
                        experiment={entry.before}
                    />
                )}
            </div>
            {open && (
                <div className="flex flex-col gap-3 px-4 pb-4">
                    {entry.status === "changed"
                        ? <ExperimentChangesView entry={entry} />
                        : <ExperimentConfigs experiment={latestOf(entry)} />}
                </div>
            )}
        </>
    );
}

interface BuildLinkProps {
    hash: TBundleHash;
}

function BuildLink({ hash }: BuildLinkProps) {
    return (
        <Link
            to="/e/view/{-$buildHash}/{-$moduleId}"
            params={{
                buildHash: hash,
                moduleId: null,
            }}
            preload={false}
            title={hash}
            className="text-primary-400 hover:underline"
        >
            <code>{hash.slice(0, 9)}</code>
        </Link>
    );
}

export function ExperimentDiff() {
    const navigate = Route.useNavigate();
    const { hashA, hashB } = Route.useParams();
    const { before, after } = Route.useLoaderData();
    const [filter, setFilter] = useState("");
    const [enabledStatuses, setEnabledStatuses] = useState<ReadonlySet<DiffStatus>>(() => new Set(DIFF_STATUSES));
    const [enabledTypes, setEnabledTypes] = useState<ReadonlySet<ExperimentType>>(() => new Set(EXPERIMENT_TYPES));
    const [enabledScopes, setEnabledScopes] = useState<ReadonlySet<ExperimentScope>>(() => new Set(EXPERIMENT_SCOPES));
    const deferredFilter = useDeferredValue(filter);
    const { entries, unchanged } = useMemo(() => diffExperiments(before, after), [before, after]);
    const sorted = useMemo(() => sortByDate(entries), [entries]);
    const statusCounts = useMemo(() => countBy(entries, DIFF_STATUSES, ({ status }) => status), [entries]);
    const typeCounts = useMemo(() => countBy(entries, EXPERIMENT_TYPES, (entry) => latestOf(entry).type), [entries]);
    const scopeCounts = useMemo(() => countBy(entries, EXPERIMENT_SCOPES, (entry) => latestOf(entry).scope), [entries]);

    const visible = useMemo(() => {
        const needle = deferredFilter.trim().toLowerCase();

        return new Set(sorted.filter((entry) => {
            const { type, scope, name, label } = latestOf(entry);

            if (!enabledStatuses.has(entry.status) || !enabledTypes.has(type) || !enabledScopes.has(scope)) {
                return false;
            }

            return !needle || name.toLowerCase().includes(needle) || label?.toLowerCase().includes(needle);
        }));
    }, [sorted, deferredFilter, enabledStatuses, enabledTypes, enabledScopes]);

    return (
        <div className="flex h-dvh flex-col bg-bg-100">
            <div className="flex flex-wrap items-center gap-3 border-b border-fg-700 p-2">
                <Text
                    size="lg"
                    weight="semiBold"
                >
                    Experiment diff
                </Text>
                <BuildLink hash={hashA} />
                <ArrowRightIcon className="size-4" />
                <BuildLink hash={hashB} />
                <IconButton
                    label="Swap builds"
                    colorType="text"
                    tooltipPosition={TooltipPosition.BOTTOM}
                    onClick={async () => {
                        await navigate({
                            params: {
                                hashA: hashB,
                                hashB: hashA,
                            },
                        });
                        return null;
                    }}
                >
                    <ArrowLeftRightIcon />
                </IconButton>
                <Text color="white-600">
                    {unchanged} unchanged
                </Text>
            </div>
            <div className="flex flex-wrap items-center gap-4 p-2">
                <Input
                    placeholder="Filter by name or label"
                    className="w-96"
                    value={filter}
                    onChange={(e) => {
                        setFilter(e.target.value);
                    }}
                    onClear={() => {
                        setFilter("");
                    }}
                    clearButton
                />
                <FilterPills
                    keys={DIFF_STATUSES}
                    labels={diffStatusLabels}
                    counts={statusCounts}
                    enabled={enabledStatuses}
                    color={diffStatusColors}
                    onToggle={(status) => {
                        setEnabledStatuses((prev) => toggledSet(prev, status));
                    }}
                />
                <FilterPills
                    keys={EXPERIMENT_TYPES}
                    labels={experimentTypeLabels}
                    counts={typeCounts}
                    enabled={enabledTypes}
                    color="accent"
                    onToggle={(type) => {
                        setEnabledTypes((prev) => toggledSet(prev, type));
                    }}
                />
                <FilterPills
                    keys={EXPERIMENT_SCOPES}
                    labels={experimentScopeLabels}
                    counts={scopeCounts}
                    enabled={enabledScopes}
                    color="secondary"
                    onToggle={(scope) => {
                        setEnabledScopes((prev) => toggledSet(prev, scope));
                    }}
                />
                <Text color="white-600">
                    {visible.size} / {entries.length} changes
                </Text>
            </div>
            {entries.length === 0
                ? (
                    <Text
                        size="3xl"
                        weight="bold"
                        center
                    >
                        No experiment changes between these builds
                    </Text>
                )
                : (
                    <ScrollArea className="min-h-0 grow">
                        <ul>
                            {sorted.map((entry) => (
                                <li
                                    key={entry.name}
                                    className="border-b border-fg-700 [content-visibility:auto]"
                                    hidden={!visible.has(entry)}
                                >
                                    <DiffRow entry={entry} />
                                </li>
                            ))}
                        </ul>
                    </ScrollArea>
                )}
        </div>
    );
}
