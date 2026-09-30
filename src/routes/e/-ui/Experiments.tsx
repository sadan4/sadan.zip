import { Button, IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { Codeblock } from "@/components/Codeblock";
import { Input } from "@/components/Input";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { Text, type TextProps } from "@/components/Text";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { Language } from "@/utils/textmate";
import type { TModuleId } from "@/utils/types";
import type { ExperimentInfo, ExperimentScope } from "@sadan4/libsadancore";
import { useQuery } from "@tanstack/react-query";

import { ModuleViewerStore, useModuleViewerSettingsStore, useModuleViewerStore, ViewMode } from "../-data";
import { Route } from "../view.{-$buildHash}.{-$moduleId}";

import { ChevronDownIcon, ChevronRightIcon, SquareArrowOutUpRightIcon } from "lucide-react";
import { useDeferredValue, useMemo, useState } from "react";

type ExperimentType = ExperimentInfo["type"];

const EXPERIMENT_TYPES: readonly ExperimentType[] = ["apex", "normal"];

const experimentTypeLabels: Record<ExperimentType, string> = {
    apex: "Apex",
    normal: "Normal",
};

const EXPERIMENT_SCOPES: readonly ExperimentScope[] = ["user", "guild", "installation"];

const experimentScopeLabels: Record<ExperimentScope, string> = {
    user: "User",
    guild: "Guild",
    installation: "Installation",
};

/**
 * experiment names start with their creation date, e.g. `2026-04-foo` or `2025-09_bar`,
 * sometimes with a day, e.g. `2026-04-12-foo`
 */
const EXPERIMENT_DATE_REGEX = /^(\d{4})-(\d{2})(?:-(\d{2}))?(?=[-_])/;

/**
 * a sortable number for the date in an experiment's name, or `-1` if it has none
 */
function experimentDateKey(name: string): number {
    const match = EXPERIMENT_DATE_REGEX.exec(name);

    if (!match) {
        return -1;
    }

    const [, year, month, day = "0"] = match;

    return (((+year * 100) + +month) * 100) + +day;
}

/**
 * newest first, undated experiments last
 */
function sortByDate(experiments: readonly ExperimentInfo[]): ExperimentInfo[] {
    return experiments
        .map((experiment) => [experimentDateKey(experiment.name), experiment] as const)
        .toSorted(([a], [b]) => b - a)
        .map(([, experiment]) => experiment);
}

function toggledSet<T>(set: ReadonlySet<T>, value: T): ReadonlySet<T> {
    const next = new Set(set);

    if (!next.delete(value)) {
        next.add(value);
    }

    return next;
}

function countBy<T, K extends string>(items: readonly T[], keys: readonly K[], getKey: (item: T) => K) {
    const counts = Object.fromEntries(keys.map((key) => [key, 0])) as Record<K, number>;

    for (const item of items) {
        counts[getKey(item)]++;
    }

    return counts;
}

interface FilterPillsProps<K extends string> {
    keys: readonly K[];
    labels: Record<K, string>;
    counts: Record<K, number>;
    enabled: ReadonlySet<K>;
    color: "accent" | "secondary";
    onToggle(key: K): void;
}

function FilterPills<K extends string>({ keys, labels, counts, enabled, color, onToggle }: FilterPillsProps<K>) {
    return (
        <div className="flex gap-2">
            {keys.map((key) => (
                <Button
                    key={key}
                    size="sm"
                    color={color}
                    colorType={enabled.has(key) ? "filled" : "outline"}
                    className="rounded-full"
                    aria-pressed={enabled.has(key)}
                    onClick={() => {
                        onToggle(key);
                    }}
                >
                    {labels[key]} ({counts[key]})
                </Button>
            ))}
        </div>
    );
}

function formatConfig(config: unknown): string {
    return JSON.stringify(config, null, 4);
}

interface BadgeProps {
    children: string;
    textColor: TextProps["color"];
}

function Badge({ children, textColor }: BadgeProps) {
    return (
        <Text
            tag="span"
            size="xs"
            color={textColor}
            className="rounded-sm border border-fg-700 px-1"
        >
            {children}
        </Text>
    );
}

interface ConfigBlockProps {
    title: string;
    config: unknown;
}

function ConfigBlock({ title, config }: ConfigBlockProps) {
    const editorTheme = useModuleViewerSettingsStore(({ editorTheme }) => editorTheme);

    return (
        <div className="flex flex-col gap-1">
            <Text
                size="sm"
                weight="bold"
            >
                {title}
            </Text>
            <Codeblock
                lang={Language.JSON}
                theme={editorTheme}
            >
                {formatConfig(config)}
            </Codeblock>
        </div>
    );
}

interface ExperimentRowProps {
    experiment: ExperimentInfo;
}

function ExperimentRow({ experiment }: ExperimentRowProps) {
    const navigate = Route.useNavigate();
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const [open, setOpen] = useState(false);
    const { moduleId, rawIndex, type, scope, name, label, defaultConfig, variants } = experiment;

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
                <IconButton
                    label={`Go to ${moduleId}.js`}
                    colorType="text"
                    tooltipPosition={TooltipPosition.LEFT}
                    onClick={async () => {
                        const location = await buildService.getSearchLocation(moduleId as TModuleId, rawIndex);

                        await navigate({
                            to: "/e/view/{-$buildHash}/{-$moduleId}",
                            params: {
                                moduleId,
                            },
                            search: {
                                sl: location.lineNumber,
                                sc: location.column,
                            },
                        });
                        ModuleViewerStore.getState().updateActivePanel(ViewMode.CODE);
                        return null;
                    }}
                >
                    <SquareArrowOutUpRightIcon />
                </IconButton>
            </div>
            {open && (
                <div className="flex flex-col gap-3 px-4 pb-4">
                    <ConfigBlock
                        title="Default Config"
                        config={defaultConfig}
                    />
                    {variants.map(({ key, label: variantLabel, config }) => (
                        <ConfigBlock
                            key={key}
                            title={variantLabel != null ? `${key}: ${variantLabel}` : key}
                            config={config}
                        />
                    ))}
                </div>
            )}
        </>
    );
}

export function ExperimentList() {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const [filter, setFilter] = useState("");
    const [enabledTypes, setEnabledTypes] = useState<ReadonlySet<ExperimentType>>(() => new Set(EXPERIMENT_TYPES));
    const [enabledScopes, setEnabledScopes] = useState<ReadonlySet<ExperimentScope>>(() => new Set(EXPERIMENT_SCOPES));
    const deferredFilter = useDeferredValue(filter);

    const { data: experiments, status, error } = useQuery({
        queryKey: ["experiments", buildHash],
        queryFn() {
            return buildService.getExperiments();
        },
    });

    const sorted = useMemo(() => {
        return sortByDate(experiments ?? []);
    }, [experiments]);

    const typeCounts = useMemo(() => {
        return countBy(experiments ?? [], EXPERIMENT_TYPES, ({ type }) => type);
    }, [experiments]);

    const scopeCounts = useMemo(() => {
        return countBy(experiments ?? [], EXPERIMENT_SCOPES, ({ scope }) => scope);
    }, [experiments]);

    const visible = useMemo(() => {
        const needle = deferredFilter.trim().toLowerCase();

        return new Set(sorted.filter(({ type, scope, name, label }) => {
            if (!enabledTypes.has(type) || !enabledScopes.has(scope)) {
                return false;
            }

            return !needle || name.toLowerCase().includes(needle) || label?.toLowerCase().includes(needle);
        }));
    }, [sorted, deferredFilter, enabledTypes, enabledScopes]);

    if (status === "pending") {
        return (
            <Text
                size="3xl"
                weight="bold"
                center
            >
                Loading Experiments...
            </Text>
        );
    }

    if (status === "error") {
        return (
            <Text
                color="error"
                center
            >
                Failed to load experiments: {String(error)}
            </Text>
        );
    }

    if (experiments.length === 0) {
        return (
            <Text
                size="3xl"
                weight="bold"
                center
            >
                No experiments found in this build
            </Text>
        );
    }

    return (
        <div className="flex size-full flex-col bg-bg-100">
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
                    {visible.size} / {experiments.length} experiments
                </Text>
            </div>
            <ScrollArea className="min-h-0 grow">
                <ul>
                    {sorted.map((experiment) => (
                        <li
                            key={experiment.name}
                            className="border-b border-fg-700 [content-visibility:auto]"
                            hidden={!visible.has(experiment)}
                        >
                            <ExperimentRow experiment={experiment} />
                        </li>
                    ))}
                </ul>
            </ScrollArea>
        </div>
    );
}
