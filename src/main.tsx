import { installF8Break } from "@/utils/devtools";

import "./index.css";
import { lazy, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Route, Routes } from "react-router";

installF8Break();

const routes = {
    index: lazy(() => import("./pages")),
    demangler: {
        index: lazy(() => import("./pages/demangler")),
    },
    minky: {
        index: lazy(() => import("./pages/minky")),
    },
    components: {
        index: lazy(() => import("./pages/components")),
    },
};

createRoot(document.body)
    .render((
        <StrictMode>
            <BrowserRouter>
                <Routes>
                    <Route
                        path="/"
                        HydrateFallback={() => null}
                    >
                        <Route
                            index
                            element={<routes.index />}
                        />
                        <Route
                            path="demangler"
                            element={<routes.demangler.index />}
                        />
                        <Route
                            path="minky"
                            element={<routes.minky.index />}
                        />
                        <Route
                            path="components"
                            element={<routes.components.index />}
                        />
                    </Route>
                </Routes>
            </BrowserRouter>
        </StrictMode>
    ));
