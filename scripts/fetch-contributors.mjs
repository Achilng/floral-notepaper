import { existsSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUTPUT = resolve(__dirname, "../src/generated/contributors.json");

const REPO = "Achilng/floral-notepaper";
const API_URL = `https://api.github.com/repos/${REPO}/contributors?per_page=100`;

async function fetchContributors() {
  const headers = { "User-Agent": "floral-notepaper-build" };
  if (process.env.GITHUB_TOKEN) {
    headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  }

  const res = await fetch(API_URL, { headers });
  if (!res.ok) {
    throw new Error(`GitHub API responded ${res.status}: ${res.statusText}`);
  }

  const data = await res.json();
  return data
    .filter((u) => u.type !== "Bot")
    .map((u) => ({
      login: u.login,
      avatar_url: u.avatar_url,
      html_url: u.html_url,
    }));
}

try {
  const contributors = await fetchContributors();
  mkdirSync(dirname(OUTPUT), { recursive: true });
  writeFileSync(OUTPUT, JSON.stringify(contributors, null, 2) + "\n");
  console.log(`[contributors] wrote ${contributors.length} contributors`);
} catch (err) {
  if (existsSync(OUTPUT)) {
    const cached = JSON.parse(readFileSync(OUTPUT, "utf-8"));
    console.warn(
      `[contributors] API failed (${err.message}), keeping cached ${cached.length} contributors`,
    );
  } else {
    writeFileSync(OUTPUT, "[]\n");
    console.warn(`[contributors] API failed (${err.message}), wrote empty fallback`);
  }
}
