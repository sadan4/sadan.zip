import App from "@/pages/index.tsx";
import { installF8Break } from "@/utils/devtools";

import "./index.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

installF8Break();

createRoot(document.body)
    .render((
        <StrictMode>
            <App />
        </StrictMode>
    ));
