import { beforeAll } from "vitest";
import { initializeI18n } from "./index";

beforeAll(async () => {
  await initializeI18n("zh-CN");
});
