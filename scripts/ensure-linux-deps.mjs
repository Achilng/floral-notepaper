import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "..");
const devDir = path.join(rootDir, "src-tauri", ".fedora-dev");
const libDir = path.join(devDir, "lib64");
const pkgconfigDir = path.join(devDir, "pkgconfig");

if (process.platform === "linux") {
  const systemLibDirs = [
    "/usr/lib64",
    "/lib64",
    "/usr/lib/x86_64-linux-gnu",
    "/usr/lib/aarch64-linux-gnu",
    "/usr/lib",
  ];
  const findSystemLib = (candidates) => {
    for (const dir of systemLibDirs) {
      for (const name of candidates) {
        const full = path.join(dir, name);
        if (fs.existsSync(full)) {
          return full;
        }
      }
    }
    return null;
  };

  fs.mkdirSync(libDir, { recursive: true });
  fs.mkdirSync(pkgconfigDir, { recursive: true });

  const symlinks = [
    { link: "libsoup-3.0.so", candidates: ["libsoup-3.0.so", "libsoup-3.0.so.0"] },
    {
      link: "libjavascriptcoregtk-4.1.so",
      candidates: ["libjavascriptcoregtk-4.1.so", "libjavascriptcoregtk-4.1.so.0"],
    },
    {
      link: "libwebkit2gtk-4.1.so",
      candidates: ["libwebkit2gtk-4.1.so", "libwebkit2gtk-4.1.so.0"],
    },
    { link: "librsvg-2.so", candidates: ["librsvg-2.so", "librsvg-2.so.2"] },
    {
      link: "libappindicator3.so",
      candidates: [
        "libappindicator3.so",
        "libappindicator3.so.1",
        "libayatana-appindicator3.so",
        "libayatana-appindicator3.so.1",
      ],
    },
    {
      link: "libappindicator3.so.1",
      candidates: [
        "libappindicator3.so.1",
        "libappindicator3.so",
        "libayatana-appindicator3.so.1",
        "libayatana-appindicator3.so",
      ],
    },
    {
      link: "libayatana-appindicator3.so",
      candidates: [
        "libayatana-appindicator3.so",
        "libayatana-appindicator3.so.1",
        "libappindicator3.so",
        "libappindicator3.so.1",
      ],
    },
    {
      link: "libayatana-appindicator3.so.1",
      candidates: [
        "libayatana-appindicator3.so.1",
        "libayatana-appindicator3.so",
        "libappindicator3.so.1",
        "libappindicator3.so",
      ],
    },
  ];

  for (const { link, candidates } of symlinks) {
    const target = findSystemLib(candidates);
    if (!target) continue;
    const dest = path.join(libDir, link);
    try {
      fs.unlinkSync(dest);
    } catch {
      // ignore
    }
    fs.symlinkSync(target, dest);
  }

  const pcFiles = {
    "libsoup-3.0.pc": `prefix=/usr
exec_prefix=\${prefix}
libdir=${libDir}
includedir=\${prefix}/include

Name: libsoup
Description: GObject-based HTTP library
Version: 3.6.0
Requires: glib-2.0 gobject-2.0 gio-2.0
Libs: -L\${libdir} -lsoup-3.0
Cflags: -I\${includedir}/libsoup-3.0
`,
    "javascriptcoregtk-4.1.pc": `prefix=/usr
exec_prefix=\${prefix}
libdir=${libDir}
includedir=\${prefix}/include

Name: JavaScriptCoreGTK+
Description: GTK+ version of the JavaScriptCore engine
Version: 2.50.0
Requires: glib-2.0 gobject-2.0
Libs: -L\${libdir} -ljavascriptcoregtk-4.1
Cflags: -I\${includedir}/webkitgtk-4.1
`,
    "webkit2gtk-4.1.pc": `prefix=/usr
exec_prefix=\${prefix}
libdir=${libDir}
includedir=\${prefix}/include

Name: WebKitGTK
Description: Web content engine for GTK
Version: 2.50.0
Requires: glib-2.0 gobject-2.0 gtk+-3.0 libsoup-3.0 javascriptcoregtk-4.1
Libs: -L\${libdir} -lwebkit2gtk-4.1
Cflags: -I\${includedir}/webkitgtk-4.1
`,
    "librsvg-2.0.pc": `prefix=/usr
exec_prefix=\${prefix}
libdir=${libDir}
includedir=\${prefix}/include

Name: librsvg
Description: library that renders svg files
Version: 2.60.0
Requires: glib-2.0 gio-2.0 gdk-pixbuf-2.0 cairo
Libs: -L\${libdir} -lrsvg-2
Cflags: -I\${includedir}/librsvg-2.0
`,
    "appindicator3-0.1.pc": `prefix=/usr
exec_prefix=\${prefix}
libdir=${libDir}
includedir=\${prefix}/include

Name: appindicator3-0.1
Description: Application indicators
Version: 12.10.1
Requires: gtk+-3.0
Libs: -L\${libdir} -lappindicator3
Cflags: -I\${includedir}/libappindicator3-0.1
`,
    "ayatana-appindicator3-0.1.pc": `prefix=/usr
exec_prefix=\${prefix}
libdir=${libDir}
includedir=\${prefix}/include

Name: ayatana-appindicator3-0.1
Description: Ayatana Application Indicators
Version: 0.5.93
Requires: gtk+-3.0
Libs: -L\${libdir} -layatana-appindicator3
Cflags: -I\${includedir}/libayatana-appindicator3-0.1
`,
  };

  for (const [filename, content] of Object.entries(pcFiles)) {
    fs.writeFileSync(path.join(pkgconfigDir, filename), content, "utf8");
  }
}

const args = process.argv.slice(2);
if (args.length > 0) {
  const env = { ...process.env };
  if (process.platform === "linux") {
    env.PKG_CONFIG_PATH = [pkgconfigDir, env.PKG_CONFIG_PATH].filter(Boolean).join(":");
    env.LIBRARY_PATH = [libDir, env.LIBRARY_PATH].filter(Boolean).join(":");
  }
  const result = spawnSync(args[0], args.slice(1), {
    stdio: "inherit",
    env,
    shell: process.platform === "win32",
  });
  process.exit(result.status ?? 1);
}
