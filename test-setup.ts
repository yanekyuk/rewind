import { GlobalRegistrator } from "@happy-dom/global-registrator";
GlobalRegistrator.register();

import { expect } from "bun:test";
import * as matchers from "@testing-library/jest-dom/matchers";
expect.extend(matchers);
