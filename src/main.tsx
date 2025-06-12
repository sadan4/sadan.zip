import { installF8Break } from "./utils/f8Break.ts";
import App from "./App.tsx";

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
