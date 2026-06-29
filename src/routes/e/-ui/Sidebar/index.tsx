import { ToggleButtonGroup } from "@/components/ToggleButtonGroup";
import { visibleIf } from "@/utils/react";

import { ModuleList } from "./ModuleList";
import { ModuleSearch } from "./Search";

import { FilesIcon, SearchIcon } from "lucide-react";
import { Activity, useState } from "react";

const enum SidebarTab {
    MODULES,
    SEARCH,
}

export function ExplorerSidebar() {
    const [tab, setTab] = useState(SidebarTab.MODULES);

    return (
        <div className="flex w-72 shrink-0 flex-col">
            <div className="flex items-center justify-center">
                <ToggleButtonGroup<SidebarTab>
                    className="m-2 mb-0 rounded-lg border-2 border-fg-700 p-2"
                    selectedItem={tab}
                    onSelectItem={setTab}
                    items={[
                        {
                            id: SidebarTab.MODULES,
                            label: "Modules",
                            renderIcon() {
                                return <FilesIcon />;
                            },
                        },
                        {
                            id: SidebarTab.SEARCH,
                            label: "Search",
                            renderIcon() {
                                return <SearchIcon />;
                            },
                        },
                    ]}
                />
            </div>
            <div className="flex min-h-0 grow">
                <Activity mode={visibleIf(tab === SidebarTab.MODULES)}>
                    <ModuleList />
                </Activity>
                <Activity mode={visibleIf(tab === SidebarTab.SEARCH)} >
                    <ModuleSearch />
                </Activity>
            </div>
        </div>
    );
}

