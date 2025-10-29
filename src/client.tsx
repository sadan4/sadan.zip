import { RouterClient } from "@tanstack/react-router/ssr/client";

import { makeRouter } from "./router";

import { hydrateRoot } from "react-dom/client";

const router = makeRouter();

hydrateRoot(document, <RouterClient router={router} />);
