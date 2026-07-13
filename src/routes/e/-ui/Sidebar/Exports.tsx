import { TreeAccordion } from "@/components/ASTViewer/TreeAccordion";
import { Clickable } from "@/components/Clickable";
import { Text } from "@/components/Text";
import type { TModuleId } from "@/utils/types";
import type { ExportTreeNode } from "@sadan4/libsadancore";
import { useQuery } from "@tanstack/react-query";

import { useModuleViewerStore } from "../../-data";
import { Route } from "../../view.{-$buildHash}.{-$moduleId}";

import { useState } from "react";

export function ModuleExports() {
    const buildHash = useModuleViewerStore(({ buildHash }) => buildHash);
    const buildService = useModuleViewerStore(({ _buildService }) => _buildService);
    const selectedModule = useModuleViewerStore(({ selectedModule }) => selectedModule);

    const { data, status, error } = useQuery({
        queryKey: ["ExplorerSidebarExportMap", buildHash, selectedModule],
        enabled: selectedModule !== null,
        queryFn() {
            return buildService.getModuleExportMap(selectedModule! as TModuleId);
        },
    });

    if (selectedModule === null) {
        return (
            <Text className="p-2">
                Select a module to view its exports
            </Text>
        );
    }

    switch (status) {
        case "pending": {
            return (
                <Text className="p-2">
                    Loading exports...
                </Text>
            );
        }
        case "error": {
            return (
                <Text className="p-2" color="error">
                    Failed to load export map:
                    {" "}
                    {error instanceof Error ? error.message : String(error)}
                </Text>
            );
        }
        case "success": {
            if (!data.length) {
                return (
                    <Text className="p-2">
                        No exports found for this module.
                    </Text>
                );
            }
            return (
                <div className="flex min-h-0 grow flex-col overflow-auto p-2">
                    {data.map((node) => (
                        <ExportTreeItem
                            key={node.name}
                            node={node}
                            moduleId={selectedModule}
                        />
                    ))}
                </div>
            );
        }
    }
}

interface ExportTreeItemProps {
    node: ExportTreeNode;
    moduleId: number;
}

function ExportTreeItem({ node, moduleId }: ExportTreeItemProps) {
    const [open, setOpen] = useState(false);
    const navigate = Route.useNavigate();
    const hasChildren = node.children.length > 0;

    function goToRange(range: ExportTreeNode["ranges"][number]) {
        navigate({
            to: "/e/view/{-$buildHash}/{-$moduleId}",
            params: {
                moduleId,
            },
            search: {
                sl: range.start.line,
                sc: range.start.column,
            },
        });
    }

    const label = (
        <div
            className="flex w-fit flex-col"
            title={node.hover}
        >
            <Text size="sm">
                {node.name}
                {node.hover && (
                    <Text
                        tag="span"
                        size="xs"
                        color="neutral-content"
                        className="ml-1"
                    >
                        {node.hover}
                    </Text>
                )}
            </Text>
            {node.ranges.map((range, i) => (
                <Clickable
                    // eslint-disable-next-line react/no-array-index-key
                    key={i}
                    onClick={() => goToRange(range)}
                    className="ml-4"
                >
                    <Text
                        size="xs"
                        color="accent"
                    >
                        {range.start.line}
                        :
                        {range.start.column}
                    </Text>
                </Clickable>
            ))}
        </div>
    );

    if (!hasChildren) {
        return label;
    }

    return (
        <TreeAccordion
            open={open}
            onArrowClick={() => setOpen((v) => !v)}
            contents={(
                <div className="flex h-fit">
                    <div className="w-4 shrink-0" />
                    <div className="grow">
                        {node.children.map((child) => (
                            <ExportTreeItem
                                key={child.name}
                                node={child}
                                moduleId={moduleId}
                            />
                        ))}
                    </div>
                </div>
            )}
        >
            {label}
        </TreeAccordion>
    );
}
