import { Button, IconButton } from "@/components/Button";
import { Clickable } from "@/components/Clickable";
import { Input } from "@/components/Input";
import { ScrollArea } from "@/components/layout/ScrollArea";
import { Text } from "@/components/Text";
import { TooltipPosition } from "@/components/Tooltip/constants";
import { copy } from "@/utils/clipboard";
import cn from "@/utils/cn";
import { NBSP } from "@/utils/constants";
import { useQuery } from "@tanstack/react-query";

import { ModuleViewerStore, useModuleViewerStore, ViewMode } from "../-data";
import type { IIcon } from "../-data/worker/sharedWorker";
import { Route } from "../view.{-$buildHash}.{-$moduleId}";

import { CopyIcon, DownloadIcon, FileCodeIcon, SquareArrowOutUpRightIcon, XIcon } from "lucide-react";
import { useDeferredValue, useMemo, useState } from "react";

const ICON_SIZES = [24, 32, 48, 64] as const;

type IconSize = typeof ICON_SIZES[number];

const DEFAULT_COLOR = "#ffffff";

interface IconEntry {
    key: string;
    icon: IIcon;
    src: string;
}

function iconKey(icon: IIcon) {
    return `${icon.definedIn}:${icon.pos.startColumn}:${icon.pos.startLineNumber}:${icon.pos.endColumn}:${icon.pos.endLineNumber}`;
}

function iconLabel(icon: IIcon) {
    return icon.name ?? `Unnamed (${icon.definedIn}.js)`;
}

/**
 * makes the scraped markup a standalone svg document with `currentColor` replaced by {@link color}
 */
function standaloneSvg(svg: string, color: string): string {
    let ret = svg.replaceAll("currentColor", color);

    if (ret.startsWith("<svg") && !/^<svg[^>]*\sxmlns=/.test(ret)) {
        ret = `<svg xmlns="http://www.w3.org/2000/svg"${ret.slice("<svg".length)}`;
    }
    return ret;
}

/**
 * The markup comes from the bundle, so render it through an `<img>` where scripts and event handlers can't run
 */
function svgDataUri(svg: string, color: string): string {
    return `data:image/svg+xml,${encodeURIComponent(standaloneSvg(svg, color))}`;
}

/**
 * named icons sorted by name, unnamed ones last
 */
function sortIcons(icons: readonly IIcon[]): IIcon[] {
    return icons.toSorted((a, b) => {
        if (a.name == null || b.name == null) {
            return +(a.name == null) - +(b.name == null);
        }
        return a.name.localeCompare(b.name);
    });
}

function downloadSvg(svg: string, name: string) {
    const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
    const a = document.createElement("a");

    a.href = url;
    a.download = `${name}.svg`;
    a.click();
    URL.revokeObjectURL(url);
}

interface IconTileProps {
    entry: IconEntry;
    size: IconSize;
    selected: boolean;
    onSelect(): void;
}

function IconTile({ entry: { icon, src }, size, selected, onSelect }: IconTileProps) {
    return (
        <Clickable
            className={cn(
                "flex w-full flex-col items-center gap-2 rounded-md border-2 p-2",
                selected ? "border-primary-400 bg-bg-200" : "border-transparent hover:border-fg-700",
            )}
            onClick={onSelect}
        >
            <img
                src={src}
                alt={iconLabel(icon)}
                width={size}
                height={size}
                loading="lazy"
                decoding="async"
                draggable={false}
            />
            <Text
                size="xs"
                color={icon.name == null ? "white-600" : undefined}
                className="w-full truncate text-center"
            >
                {icon.name ?? "Unnamed"}
            </Text>
        </Clickable>
    );
}

interface IconDetailsProps {
    entry: IconEntry;
    color: string;
    onClose(): void;
}

function IconDetails({ entry: { icon, src }, color, onClose }: IconDetailsProps) {
    const navigate = Route.useNavigate();
    const { name, definedIn, pos, svg } = icon;
    const fileName = name ?? `icon-${definedIn}`;

    return (
        <div className="flex w-80 shrink-0 flex-col gap-4 border-l-2 border-fg-700 p-4">
            <div className="flex items-center justify-between gap-2">
                <Text
                    size="lg"
                    weight="bold"
                    className="min-w-0 break-all"
                >
                    {iconLabel(icon)}
                </Text>
                <IconButton
                    label="Close"
                    colorType="text"
                    color="error"
                    tooltipPosition={TooltipPosition.LEFT}
                    onClick={() => {
                        onClose();
                        return null;
                    }}
                >
                    <XIcon />
                </IconButton>
            </div>
            <div className="flex items-center justify-center rounded-md border-2 border-fg-700 bg-bg-200 p-6">
                <img
                    src={src}
                    alt={iconLabel(icon)}
                    width={128}
                    height={128}
                    draggable={false}
                />
            </div>
            <Text
                size="sm"
                color="white-600"
            >
                Defined in {definedIn}.js at line {pos.startLineNumber}
            </Text>
            <div className="flex flex-wrap gap-2">
                {name != null && (
                    <IconButton
                        label={`Copy${NBSP}Name`}
                        colorType="outline"
                        tooltipPosition={TooltipPosition.BOTTOM}
                        onClick={() => copy(name).then(() => true)}
                    >
                        <CopyIcon />
                    </IconButton>
                )}
                <IconButton
                    label={`Copy${NBSP}SVG`}
                    colorType="outline"
                    tooltipPosition={TooltipPosition.BOTTOM}
                    onClick={() => copy(standaloneSvg(svg, "currentColor")).then(() => true)}
                >
                    <FileCodeIcon />
                </IconButton>
                <IconButton
                    label={`Download${NBSP}SVG`}
                    colorType="outline"
                    tooltipPosition={TooltipPosition.BOTTOM}
                    onClick={() => {
                        downloadSvg(standaloneSvg(svg, color), fileName);
                        return true;
                    }}
                >
                    <DownloadIcon />
                </IconButton>
                <IconButton
                    label={`Go${NBSP}to${NBSP}${definedIn}.js`}
                    colorType="outline"
                    tooltipPosition={TooltipPosition.BOTTOM}
                    onClick={async () => {
                        await navigate({
                            to: "/e/view/{-$buildHash}/{-$moduleId}",
                            params: {
                                moduleId: definedIn,
                            },
                            search: {
                                sl: pos.startLineNumber,
                                sc: pos.startColumn,
                                el: pos.endLineNumber,
                                ec: pos.endColumn,
                            },
                        });
                        ModuleViewerStore.getState().updateActivePanel(ViewMode.CODE);
                        return null;
                    }}
                >
                    <SquareArrowOutUpRightIcon />
                </IconButton>
            </div>
        </div>
    );
}

export function IconBrowser() {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const [filter, setFilter] = useState("");
    const [color, setColor] = useState(DEFAULT_COLOR);
    const [size, setSize] = useState<IconSize>(32);
    const [showUnnamed, setShowUnnamed] = useState(true);
    const [selectedKey, setSelectedKey] = useState<string | null>(null);
    const deferredFilter = useDeferredValue(filter);
    const deferredColor = useDeferredValue(color);

    const { data: icons, status, error } = useQuery({
        queryKey: ["icons", buildHash],
        queryFn() {
            return buildService.getIcons();
        },
    });

    const sorted = useMemo(() => sortIcons(icons ?? []), [icons]);

    const entries = useMemo(() => {
        return sorted.map((icon): IconEntry => ({
            key: iconKey(icon),
            icon,
            src: svgDataUri(icon.svg, deferredColor),
        }));
    }, [sorted, deferredColor]);

    const unnamedCount = useMemo(() => sorted.filter(({ name }) => name == null).length, [sorted]);

    const visible = useMemo(() => {
        const needle = deferredFilter.trim().toLowerCase();

        return new Set(entries.filter(({ icon: { name, definedIn } }) => {
            if (name == null) {
                return showUnnamed && (!needle || String(definedIn).includes(needle));
            }
            return !needle || name.toLowerCase().includes(needle);
        }));
    }, [entries, deferredFilter, showUnnamed]);

    const selected = entries.find(({ key }) => key === selectedKey);

    if (status === "pending") {
        return (
            <Text
                size="3xl"
                weight="bold"
                center
            >
                Loading Icons...
            </Text>
        );
    }

    if (status === "error") {
        return (
            <Text
                color="error"
                center
            >
                Failed to load icons: {String(error)}
            </Text>
        );
    }

    if (icons.length === 0) {
        return (
            <Text
                size="3xl"
                weight="bold"
                center
            >
                No icons found in this build
            </Text>
        );
    }

    return (
        <div className="flex size-full bg-bg-100">
            <div className="flex min-w-0 grow flex-col">
                <div className="flex flex-wrap items-center gap-4 p-2">
                    <Input
                        placeholder="Filter by name"
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
                    <div className="flex gap-2">
                        {ICON_SIZES.map((s) => (
                            <Button
                                key={s}
                                size="sm"
                                color="accent"
                                colorType={size === s ? "filled" : "outline"}
                                className="rounded-full"
                                aria-pressed={size === s}
                                onClick={() => {
                                    setSize(s);
                                }}
                            >
                                {s}px
                            </Button>
                        ))}
                    </div>
                    <Button
                        size="sm"
                        color="secondary"
                        colorType={showUnnamed ? "filled" : "outline"}
                        className="rounded-full"
                        aria-pressed={showUnnamed}
                        onClick={() => {
                            setShowUnnamed((s) => !s);
                        }}
                    >
                        Unnamed ({unnamedCount})
                    </Button>
                    <label className="flex items-center gap-2">
                        <Text tag="span">Color</Text>
                        <input
                            type="color"
                            className="h-8 w-10 cursor-pointer rounded-sm bg-transparent"
                            value={color}
                            onChange={(e) => {
                                setColor(e.target.value);
                            }}
                        />
                    </label>
                    <Text color="white-600">
                        {visible.size} / {icons.length} icons
                    </Text>
                </div>
                <ScrollArea className="min-h-0 grow">
                    <ul
                        className="grid gap-2 p-2"
                        style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${Math.max(size + 32, 96)}px, 1fr))` }}
                    >
                        {entries.map((entry) => (
                            <li
                                key={entry.key}
                                className="[content-visibility:auto]"
                                hidden={!visible.has(entry)}
                            >
                                <IconTile
                                    entry={entry}
                                    size={size}
                                    selected={entry.key === selectedKey}
                                    onSelect={() => {
                                        setSelectedKey(entry.key === selectedKey ? null : entry.key);
                                    }}
                                />
                            </li>
                        ))}
                    </ul>
                </ScrollArea>
            </div>
            {selected && (
                <IconDetails
                    entry={selected}
                    color={deferredColor}
                    onClose={() => {
                        setSelectedKey(null);
                    }}
                />
            )}
        </div>
    );
}
