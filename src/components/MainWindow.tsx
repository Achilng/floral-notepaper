import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { emit, listen } from "@tauri-apps/api/event";
import { exportMarkdownNote, importMarkdownNote } from "../features/importExport/api";
import { MarkdownPreview } from "../features/markdown/MarkdownPreview";
import {
  chooseNotesDirectory,
  getConfig,
  normalizeViewMode,
  saveConfig,
} from "../features/settings/api";
import type { AppConfig, ViewMode } from "../features/settings/types";
import { normalizeTileColor } from "../features/settings/tileColor";
import { BackgroundLayer } from "./BackgroundLayer";
import { SettingsPanel } from "./SettingsPanel";
import { SlidingButtonGroup } from "./SlidingButtonGroup";
import {
  createNote,
  createCategory,
  deleteCategory,
  deleteNote,
  getErrorMessage,
  getFileModifiedTime,
  getNote,
  listCategories,
  listNotes,
  moveNoteCategory,
  readExternalFile,
  renameCategory,
  saveExternalFile,
  updateNote,
} from "../features/notes/api";
import type { ExternalFile, Note, NoteMetadata } from "../features/notes/types";
import {
  countNoteChars,
  filterNotes,
  formatShortDate,
  formatTime,
  getDisplayTitle,
  groupNotesByCategory,
  metadataFromNote,
} from "../features/notes/noteUtils";
import type { CategoryGroup } from "../features/notes/noteUtils";
import {
  getNoteContextMenuItems,
  type NoteContextMenuAction,
} from "../features/notes/noteContextMenu";
import { openNotepadWindow, toggleTileWindow } from "../features/windows/api";
import {
  closeCurrentWindow,
  minimizeCurrentWindow,
  toggleMaximizeCurrentWindow,
  isCurrentWindowMaximized,
  startCurrentWindowDrag,
} from "../features/windows/controls";
import {
  TILE_WINDOW_CLOSED_EVENT,
  TILE_WINDOW_UNPINNED_EVENT,
  syncPinnedTileIds,
} from "../features/windows/tileWindowEvents";

type SaveState = "idle" | "dirty" | "saving" | "saved" | "error";

interface NoteMenuState {
  x: number;
  y: number;
  noteId: string;
}

interface CategoryMenuState {
  x: number;
  y: number;
  category: string;
}

type FormatAction =
  | "bold"
  | "italic"
  | "heading"
  | "hr"
  | "ul"
  | "ol"
  | "code"
  | "quote"
  | "inlineMath"
  | "blockMath";

function applyFormat(
  textarea: HTMLTextAreaElement,
  action: FormatAction,
  translate: TFunction,
  setContent: (v: string) => void,
  markDirty: () => void,
) {
  const { selectionStart: start, selectionEnd: end, value } = textarea;
  const selected = value.slice(start, end);
  const before = value.slice(0, start);
  const after = value.slice(end);

  const lineStart = before.lastIndexOf("\n") + 1;
  const currentLine = before.slice(lineStart);

  let result: string;
  let cursorStart: number;
  let cursorEnd: number;

  switch (action) {
    case "bold": {
      const fallback = translate("main.formatSample.boldText", { defaultValue: "粗体文本" });
      const wrapped = `**${selected || fallback}**`;
      result = before + wrapped + after;
      cursorStart = start + 2;
      cursorEnd = cursorStart + (selected || fallback).length;
      break;
    }
    case "italic": {
      const fallback = translate("main.formatSample.italicText", { defaultValue: "斜体文本" });
      const wrapped = `*${selected || fallback}*`;
      result = before + wrapped + after;
      cursorStart = start + 1;
      cursorEnd = cursorStart + (selected || fallback).length;
      break;
    }
    case "heading": {
      const prefix = currentLine.match(/^(#{1,5})\s/);
      if (prefix) {
        const newLevel = prefix[1].length < 5 ? "#".repeat(prefix[1].length + 1) : "#";
        const beforeLine = value.slice(0, lineStart);
        const afterPrefix = value.slice(lineStart + prefix[0].length);
        result = beforeLine + newLevel + " " + afterPrefix;
        const offset = newLevel.length + 1 - prefix[0].length;
        cursorStart = start + offset;
        cursorEnd = end + offset;
      } else if (currentLine.length > 0 && start === end) {
        result = value.slice(0, lineStart) + "## " + value.slice(lineStart);
        cursorStart = start + 3;
        cursorEnd = cursorStart;
      } else if (selected) {
        result = before + `## ${selected}` + after;
        cursorStart = start + 3;
        cursorEnd = cursorStart + selected.length;
      } else {
        result =
          before +
          `## ${translate("main.formatSample.headingText", { defaultValue: "标题" })}` +
          after;
        cursorStart = start + 3;
        cursorEnd = cursorStart + 2;
      }
      break;
    }
    case "hr": {
      const newlineBefore = before.endsWith("\n") || before === "" ? "" : "\n";
      const newlineAfter = after.startsWith("\n") || after === "" ? "" : "\n";
      result = before + `${newlineBefore}---${newlineAfter}` + after;
      cursorStart = cursorEnd = before.length + newlineBefore.length + 3;
      break;
    }
    case "ul": {
      if (selected.includes("\n")) {
        const lines = selected
          .split("\n")
          .map((l) => `- ${l}`)
          .join("\n");
        result = before + lines + after;
        cursorStart = start;
        cursorEnd = start + lines.length;
      } else {
        const fallback = translate("main.formatSample.listItem", { defaultValue: "列表项" });
        const item = `- ${selected || fallback}`;
        result = before + item + after;
        cursorStart = start + 2;
        cursorEnd = cursorStart + (selected || fallback).length;
      }
      break;
    }
    case "ol": {
      if (selected.includes("\n")) {
        const lines = selected
          .split("\n")
          .map((l, i) => `${i + 1}. ${l}`)
          .join("\n");
        result = before + lines + after;
        cursorStart = start;
        cursorEnd = start + lines.length;
      } else {
        const fallback = translate("main.formatSample.listItem", { defaultValue: "列表项" });
        const item = `1. ${selected || fallback}`;
        result = before + item + after;
        cursorStart = start + 3;
        cursorEnd = cursorStart + (selected || fallback).length;
      }
      break;
    }
    case "code": {
      if (selected.includes("\n")) {
        const wrapped = "```\n" + selected + "\n```";
        result = before + wrapped + after;
        cursorStart = start + 4;
        cursorEnd = cursorStart + selected.length;
      } else {
        const fallback = translate("main.formatSample.codeText", { defaultValue: "代码" });
        const wrapped = `\`${selected || fallback}\``;
        result = before + wrapped + after;
        cursorStart = start + 1;
        cursorEnd = cursorStart + (selected || fallback).length;
      }
      break;
    }
    case "quote": {
      if (selected.includes("\n")) {
        const lines = selected
          .split("\n")
          .map((l) => `> ${l}`)
          .join("\n");
        result = before + lines + after;
        cursorStart = start;
        cursorEnd = start + lines.length;
      } else {
        const fallback = translate("main.formatSample.quoteText", { defaultValue: "引用文本" });
        const item = `> ${selected || fallback}`;
        result = before + item + after;
        cursorStart = start + 2;
        cursorEnd = cursorStart + (selected || fallback).length;
      }
      break;
    }
    case "inlineMath": {
      const wrapped = `$${selected || "E=mc^2"}$`;
      result = before + wrapped + after;
      cursorStart = start + 1;
      cursorEnd = cursorStart + (selected || "E=mc^2").length;
      break;
    }
    case "blockMath": {
      const wrapped = `\n$$\n${selected || "x^2 + y^2 = r^2"}\n$$\n`;
      result = before + wrapped + after;
      cursorStart = start + 4;
      cursorEnd = cursorStart + (selected || "x^2 + y^2 = r^2").length;
      break;
    }
  }

  setContent(result);
  markDirty();
  requestAnimationFrame(() => {
    textarea.focus();
    textarea.setSelectionRange(cursorStart, cursorEnd);
  });
}

type UndoDocument = Pick<Document, "execCommand">;

export function runEditorUndo(
  textarea: HTMLTextAreaElement | null,
  doc: UndoDocument = document,
): boolean {
  if (!textarea || textarea.disabled) return false;
  textarea.focus();
  return doc.execCommand("undo");
}

export function pinTileButtonTitle(isPinned: boolean): string {
  return isPinned ? "取消钉屏" : "钉到屏幕";
}

interface MainWindowProps {
  initialSettingsOpen?: boolean;
  initialConfig?: AppConfig;
  initialErrorMessage?: string | null;
}

export function MainWindow({
  initialSettingsOpen = false,
  initialConfig = undefined,
  initialErrorMessage = null,
}: MainWindowProps = {}) {
  const { t } = useTranslation();
  const [notes, setNotes] = useState<NoteMetadata[]>([]);
  const [externalFiles, setExternalFiles] = useState<ExternalFile[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [viewMode, setViewMode] = useState<ViewMode>(
    normalizeViewMode(initialConfig?.defaultViewMode ?? "split"),
  );
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(initialErrorMessage);
  const [noteMenu, setNoteMenu] = useState<NoteMenuState | null>(null);
  const [noteMenuClosing, setNoteMenuClosing] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(initialSettingsOpen);
  const [settingsConfig, setSettingsConfig] = useState<AppConfig | null>(initialConfig ?? null);
  const [savedNotesDir, setSavedNotesDir] = useState<string | null>(
    initialConfig?.notesDir ?? null,
  );
  const [noteTransitionKey, setNoteTransitionKey] = useState(0);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [deleteExiting, setDeleteExiting] = useState(false);
  const [pinnedTileIds, setPinnedTileIds] = useState<Set<string>>(new Set());
  const [categories, setCategories] = useState<string[]>([]);
  const [collapsedCategories, setCollapsedCategories] = useState<Set<string>>(new Set());
  const [activeCategory, setActiveCategory] = useState<string>("");
  const [showCategoryInput, setShowCategoryInput] = useState(false);
  const [categoryInputValue, setCategoryInputValue] = useState("");
  const [noteMenuMode, setNoteMenuMode] = useState<"main" | "move">("main");
  const [renamingCategory, setRenamingCategory] = useState<string | null>(null);
  const [renameCategoryValue, setRenameCategoryValue] = useState("");
  const [dragOverCategory, setDragOverCategory] = useState<string | null>(null);
  const [settingsOverlay, setSettingsOverlay] = useState(() => window.innerWidth < 1080);
  const [sidebarWidth, setSidebarWidth] = useState(280);
  const [isResizingSidebar, setIsResizingSidebar] = useState(false);
  const [splitRatio, setSplitRatio] = useState(0.5);
  const [isResizingSplit, setIsResizingSplit] = useState(false);
  const splitContainerRef = useRef<HTMLDivElement>(null);
  const [categoryMenu, setCategoryMenu] = useState<CategoryMenuState | null>(null);
  const [categoryMenuClosing, setCategoryMenuClosing] = useState(false);
  const [categoryMenuConfirmDelete, setCategoryMenuConfirmDelete] = useState(false);
  const contentRef = useRef<HTMLTextAreaElement>(null);
  const externalFileMtimeRef = useRef<number>(0);
  const lastExternalSaveRef = useRef<number>(0);
  const saveStateRef = useRef(saveState);
  saveStateRef.current = saveState;
  const selectedIdRef = useRef(selectedId);
  selectedIdRef.current = selectedId;

  const selectedNote = useMemo(
    () => notes.find((note) => note.id === selectedId) ?? null,
    [notes, selectedId],
  );
  const selectedNoteRef = useRef(selectedNote);
  selectedNoteRef.current = selectedNote;

  const selectedExternalFile = useMemo(
    () => externalFiles.find((f) => f.id === selectedId) ?? null,
    [externalFiles, selectedId],
  );

  const isExternal = selectedExternalFile !== null;

  const noteMenuTarget = useMemo(
    () => notes.find((note) => note.id === noteMenu?.noteId) ?? null,
    [noteMenu?.noteId, notes],
  );
  const noteContextMenuItems = useMemo(() => getNoteContextMenuItems(t), [t]);
  const saveStateLabel = useMemo<Record<SaveState, string>>(
    () => ({
      idle: t("main.statusBar.saveState.idle", { defaultValue: "未选择" }),
      dirty: t("main.statusBar.saveState.dirty", { defaultValue: "未保存" }),
      saving: t("main.statusBar.saveState.saving", { defaultValue: "保存中" }),
      saved: t("main.statusBar.saveState.saved", { defaultValue: "已保存" }),
      error: t("main.statusBar.saveState.error", { defaultValue: "保存失败" }),
    }),
    [t],
  );
  const toolbarButtons = useMemo<
    { label: string; title: string; style: string; action: FormatAction }[]
  >(
    () => [
      {
        label: "B",
        title: t("main.toolbar.bold", { defaultValue: "粗体" }),
        style: "font-bold",
        action: "bold",
      },
      {
        label: "I",
        title: t("main.toolbar.italic", { defaultValue: "斜体" }),
        style: "italic",
        action: "italic",
      },
      {
        label: "H",
        title: t("main.toolbar.heading", { defaultValue: "标题" }),
        style: "font-bold",
        action: "heading",
      },
      {
        label: "—",
        title: t("main.toolbar.hr", { defaultValue: "分割线" }),
        style: "",
        action: "hr",
      },
      {
        label: "•",
        title: t("main.toolbar.ul", { defaultValue: "无序列表" }),
        style: "",
        action: "ul",
      },
      {
        label: "1.",
        title: t("main.toolbar.ol", { defaultValue: "有序列表" }),
        style: "font-mono text-[9px]",
        action: "ol",
      },
      {
        label: "<>",
        title: t("main.toolbar.code", { defaultValue: "代码" }),
        style: "font-mono text-[9px]",
        action: "code",
      },
      {
        label: "❝",
        title: t("main.toolbar.quote", { defaultValue: "引用" }),
        style: "",
        action: "quote",
      },
      {
        label: "∑",
        title: t("main.toolbar.inlineMath", { defaultValue: "行内公式" }),
        style: "font-mono text-[11px]",
        action: "inlineMath",
      },
      {
        label: "∫",
        title: t("main.toolbar.blockMath", { defaultValue: "块级公式" }),
        style: "font-mono text-[11px]",
        action: "blockMath",
      },
    ],
    [t],
  );
  const viewModeOptions = useMemo(
    () => [
      {
        value: "edit" as ViewMode,
        label: t("settings.defaultView.edit", { defaultValue: "编辑" }),
      },
      {
        value: "split" as ViewMode,
        label: t("settings.defaultView.split", { defaultValue: "分栏" }),
      },
      {
        value: "preview" as ViewMode,
        label: t("settings.defaultView.preview", { defaultValue: "预览" }),
      },
    ],
    [t],
  );

  const filteredNotes = useMemo(() => filterNotes(notes, searchQuery), [notes, searchQuery]);

  const categoryGroups = useMemo(
    () => groupNotesByCategory(filteredNotes, categories),
    [filteredNotes, categories],
  );

  const lineCount = useMemo(() => content.split("\n").length, [content]);
  const byteSize = useMemo(
    () => (new TextEncoder().encode(content).length / 1024).toFixed(1),
    [content],
  );
  const charCount = useMemo(() => countNoteChars(content), [content]);

  const applyNote = useCallback((note: Note) => {
    setSelectedId(note.id);
    setTitle(note.title);
    setContent(note.content);
    setSaveState("saved");
    setErrorMessage(null);
    setNoteTransitionKey((k) => k + 1);
  }, []);

  const replaceNoteMetadata = useCallback((note: Note) => {
    const metadata = metadataFromNote(note);
    setNotes((current) => {
      const exists = current.some((item) => item.id === metadata.id);
      const next = exists
        ? current.map((item) => (item.id === metadata.id ? metadata : item))
        : [metadata, ...current];
      return [...next].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
    });
  }, []);

  const loadNote = useCallback(
    async (id: string) => {
      setErrorMessage(null);
      const note = await getNote(id);
      applyNote(note);
      replaceNoteMetadata(note);
    },
    [applyNote, replaceNoteMetadata],
  );

  const refreshNotes = useCallback(async () => {
    const [loadedNotes, loadedCategories] = await Promise.all([listNotes(), listCategories()]);
    setNotes(loadedNotes);
    setCategories(loadedCategories);
    return loadedNotes;
  }, []);

  const clearCurrentNote = useCallback(() => {
    setSelectedId(null);
    setTitle("");
    setContent("");
    setSaveState("idle");
  }, []);

  const loadExternalFile = useCallback(async (filePath: string) => {
    setErrorMessage(null);
    try {
      const [fileContent, mtime] = await Promise.all([
        readExternalFile(filePath),
        getFileModifiedTime(filePath),
      ]);
      const fileName = filePath.split(/[\\/]/).pop() ?? filePath;
      const displayTitle = fileName.replace(/\.(md|txt)$/i, "");

      setExternalFiles((current) => {
        if (current.some((f) => f.id === filePath)) {
          return current;
        }
        return [
          ...current,
          {
            id: filePath,
            title: displayTitle,
            filePath,
          },
        ];
      });

      setSelectedId(filePath);
      setTitle(displayTitle);
      setContent(fileContent);
      setSaveState("saved");
      setNoteTransitionKey((k) => k + 1);
      externalFileMtimeRef.current = mtime;
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      setIsLoading(true);
      try {
        const [loadedConfig, loadedNotes, loadedCategories] = await Promise.all([
          getConfig(),
          listNotes(),
          listCategories(),
        ]);
        if (cancelled) return;
        setSettingsConfig(loadedConfig);
        setSavedNotesDir(loadedConfig.notesDir);
        setViewMode(normalizeViewMode(loadedConfig.defaultViewMode));
        setNotes(loadedNotes);
        setCategories(loadedCategories);
        setCollapsedCategories(new Set(loadedCategories));
        if (loadedNotes[0]) {
          const note = await getNote(loadedNotes[0].id);
          if (!cancelled) applyNote(note);
        } else {
          clearCurrentNote();
        }
      } catch (error) {
        if (!cancelled) setErrorMessage(getErrorMessage(error));
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    }

    void bootstrap();
    return () => {
      cancelled = true;
    };
  }, [applyNote, clearCurrentNote]);

  useEffect(() => {
    const unlisten = listen("notes-changed", () => {
      void refreshNotes().then((loaded) => {
        const currentId = selectedIdRef.current;
        if (!currentId) return;
        const stillExists = loaded.some((n) => n.id === currentId);
        if (stillExists) {
          if (saveStateRef.current !== "dirty") {
            void getNote(currentId)
              .then((note) => {
                if (selectedIdRef.current !== currentId) return;
                setTitle(note.title);
                setContent(note.content);
                setSaveState("saved");
              })
              .catch(() => undefined);
          }
        } else if (selectedNoteRef.current) {
          if (loaded[0]) {
            void loadNote(loaded[0].id);
          } else {
            clearCurrentNote();
          }
        }
      });
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refreshNotes, loadNote, clearCurrentNote]);

  useEffect(() => {
    function handleFocus() {
      void refreshNotes();
    }
    window.addEventListener("focus", handleFocus);
    return () => window.removeEventListener("focus", handleFocus);
  }, [refreshNotes]);

  useEffect(() => {
    const onResize = () => setSettingsOverlay(window.innerWidth < 1080);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("open-external-file", (event) => {
      void loadExternalFile(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [loadExternalFile]);

  useEffect(() => {
    const unlisten = listen<string>("open-note", (event) => {
      void loadNote(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [loadNote]);

  useEffect(() => {
    const unlisten = listen<string>("shortcut-register-failed", (event) => {
      setErrorMessage(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<string>(TILE_WINDOW_CLOSED_EVENT, (event) => {
      setPinnedTileIds((previous) => syncPinnedTileIds(previous, event.payload, false));
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<string>(TILE_WINDOW_UNPINNED_EVENT, (event) => {
      setPinnedTileIds((previous) => syncPinnedTileIds(previous, event.payload, false));
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!selectedExternalFile) return;

    const interval = window.setInterval(async () => {
      if (Date.now() - lastExternalSaveRef.current < 2000) return;
      try {
        const mtime = await getFileModifiedTime(selectedExternalFile.filePath);
        if (mtime !== externalFileMtimeRef.current) {
          externalFileMtimeRef.current = mtime;
          const fileContent = await readExternalFile(selectedExternalFile.filePath);
          setContent(fileContent);
          setSaveState("saved");
        }
      } catch {
        // file may have been deleted or become inaccessible
      }
    }, 1000);

    return () => window.clearInterval(interval);
  }, [selectedExternalFile]);

  useEffect(() => {
    function closeMenus() {
      setNoteMenuClosing(true);
      setCategoryMenuClosing(true);
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") closeMenus();
    }

    document.addEventListener("mousedown", closeMenus);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", closeMenus);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  useEffect(() => {
    if (!noteMenuClosing || !noteMenu) return;
    const timer = window.setTimeout(() => {
      setNoteMenu(null);
      setNoteMenuClosing(false);
      setNoteMenuMode("main");
    }, 150);
    return () => window.clearTimeout(timer);
  }, [noteMenuClosing, noteMenu]);

  useEffect(() => {
    if (!categoryMenuClosing || !categoryMenu) return;
    const timer = window.setTimeout(() => {
      setCategoryMenu(null);
      setCategoryMenuClosing(false);
      setCategoryMenuConfirmDelete(false);
    }, 150);
    return () => window.clearTimeout(timer);
  }, [categoryMenuClosing, categoryMenu]);

  const saveCurrentNote = useCallback(async () => {
    if (!selectedId) return null;

    if (isExternal && selectedExternalFile) {
      setSaveState("saving");
      try {
        await saveExternalFile(selectedExternalFile.filePath, content);
        lastExternalSaveRef.current = Date.now();
        const mtime = await getFileModifiedTime(selectedExternalFile.filePath);
        externalFileMtimeRef.current = mtime;
        setSaveState("saved");
        setErrorMessage(null);
        return { id: selectedId, title, content } as Note;
      } catch (error) {
        setSaveState("error");
        setErrorMessage(getErrorMessage(error));
        return null;
      }
    }

    setSaveState("saving");
    try {
      const category = selectedNote?.category ?? "";
      const note = await updateNote(selectedId, { title, content, category });
      replaceNoteMetadata(note);
      setSaveState("saved");
      setErrorMessage(null);
      return note;
    } catch (error) {
      setSaveState("error");
      setErrorMessage(getErrorMessage(error));
      return null;
    }
  }, [
    content,
    isExternal,
    replaceNoteMetadata,
    selectedExternalFile,
    selectedId,
    selectedNote,
    title,
  ]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key === "s") {
        event.preventDefault();
        void saveCurrentNote();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [saveCurrentNote]);

  useEffect(() => {
    if (!selectedId || saveState !== "dirty") return undefined;
    if (isExternal) {
      if (!settingsConfig?.externalFileAutoSave) return undefined;
    } else {
      if (!settingsConfig?.noteAutoSave) return undefined;
    }

    const timer = window.setTimeout(() => {
      void saveCurrentNote();
    }, 900);

    return () => window.clearTimeout(timer);
  }, [
    isExternal,
    saveCurrentNote,
    saveState,
    selectedId,
    settingsConfig?.noteAutoSave,
    settingsConfig?.externalFileAutoSave,
  ]);

  const handleNewNote = async () => {
    setErrorMessage(null);
    if (saveState === "dirty") {
      await saveCurrentNote();
    }
    try {
      const note = await createNote({ title: "", content: "", category: activeCategory });
      replaceNoteMetadata(note);
      applyNote(note);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleOpenSettings = async () => {
    if (settingsOpen) {
      setSettingsOpen(false);
      return;
    }
    setSettingsOpen(true);
    if (settingsConfig) return;

    setErrorMessage(null);
    try {
      const config = await getConfig();
      setSettingsConfig(config);
      setSavedNotesDir(config.notesDir);
      setViewMode(normalizeViewMode(config.defaultViewMode));
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleChooseNotesDir = async () => {
    if (!settingsConfig) return;

    setErrorMessage(null);
    try {
      const notesDir = await chooseNotesDirectory();
      if (!notesDir) return;
      handleSettingsChange({ ...settingsConfig, notesDir });
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const settingsSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const persistSettings = useCallback(
    (nextConfig: AppConfig) => {
      if (settingsSaveTimer.current) {
        clearTimeout(settingsSaveTimer.current);
      }
      settingsSaveTimer.current = setTimeout(async () => {
        const previousNotesDir = savedNotesDir ?? nextConfig.notesDir;
        const normalizedConfig = {
          ...nextConfig,
          defaultViewMode: normalizeViewMode(nextConfig.defaultViewMode),
          tileColor: normalizeTileColor(nextConfig.tileColor),
        };
        try {
          const savedConfig = await saveConfig(normalizedConfig);
          setSettingsConfig(savedConfig);
          setSavedNotesDir(savedConfig.notesDir);
          setViewMode(normalizeViewMode(savedConfig.defaultViewMode));

          if (savedConfig.notesDir !== previousNotesDir) {
            const loadedNotes = await refreshNotes();
            if (loadedNotes[0]) {
              await loadNote(loadedNotes[0].id);
            } else {
              clearCurrentNote();
            }
          }
        } catch (error) {
          setErrorMessage(getErrorMessage(error));
        }
      }, 300);
    },
    [savedNotesDir, refreshNotes, loadNote, clearCurrentNote],
  );

  const handleSettingsChange = useCallback(
    (nextConfig: AppConfig) => {
      setSettingsConfig(nextConfig);
      void emit("config-changed", nextConfig);
      persistSettings(nextConfig);
    },
    [persistSettings],
  );

  const handleCloseSettings = useCallback(() => {
    setSettingsOpen(false);
  }, []);

  const handleImportNote = async () => {
    setErrorMessage(null);
    try {
      if (selectedId && saveState === "dirty") {
        const saved = await saveCurrentNote();
        if (!saved) return;
      }

      const note = await importMarkdownNote(activeCategory);
      if (!note) return;

      replaceNoteMetadata(note);
      applyNote(note);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleSelectNote = async (id: string) => {
    if (id === selectedId) return;
    setDeleteConfirm(false);
    if (saveState === "dirty") {
      await saveCurrentNote();
    }

    setIsLoading(true);
    try {
      await loadNote(id);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectExternalFile = async (id: string) => {
    if (id === selectedId) return;
    setDeleteConfirm(false);
    if (saveState === "dirty") {
      await saveCurrentNote();
    }

    const file = externalFiles.find((f) => f.id === id);
    if (!file) return;

    setIsLoading(true);
    try {
      const [fileContent, mtime] = await Promise.all([
        readExternalFile(file.filePath),
        getFileModifiedTime(file.filePath),
      ]);
      setSelectedId(id);
      setTitle(file.title);
      setContent(fileContent);
      setSaveState("saved");
      setErrorMessage(null);
      setNoteTransitionKey((k) => k + 1);
      externalFileMtimeRef.current = mtime;
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  };

  const handleRemoveExternalFile = async (id: string) => {
    if (selectedId === id && saveState === "dirty") {
      const shouldSave = window.confirm(
        t("main.confirm.unsavedExternalFile", {
          title: title || t("common.untitledFile", { defaultValue: "未命名文件" }),
          defaultValue: "「{{title}}」有未保存的更改，是否保存到原文件？",
        }),
      );
      if (shouldSave) {
        const saved = await saveCurrentNote();
        if (!saved) return;
      }
    }
    setExternalFiles((current) => current.filter((f) => f.id !== id));
    if (selectedId === id) {
      clearCurrentNote();
    }
  };

  const handleDeleteNote = async (noteId = selectedId) => {
    if (!noteId) return;

    setDeleteConfirm(false);
    setErrorMessage(null);
    try {
      await deleteNote(noteId);
      const remaining = await refreshNotes();
      if (noteId === selectedId && remaining[0]) {
        await loadNote(remaining[0].id);
      } else if (noteId === selectedId) {
        clearCurrentNote();
      }
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleOpenNoteMenu = (event: MouseEvent<HTMLElement>, noteId: string) => {
    event.preventDefault();
    event.stopPropagation();

    const menuWidth = 168;
    const menuHeight = 76;
    const x = Math.min(event.clientX, window.innerWidth - menuWidth - 4);
    const y = Math.min(event.clientY, window.innerHeight - menuHeight - 4);

    setNoteMenuClosing(false);
    setHoveredId(noteId);
    setNoteMenu({
      x: Math.max(4, x),
      y: Math.max(4, y),
      noteId,
    });
  };

  const handleExportNote = async (note: NoteMetadata) => {
    setErrorMessage(null);
    try {
      if (note.id === selectedId && saveState === "dirty") {
        const saved = await saveCurrentNote();
        if (!saved) return;
      }

      await exportMarkdownNote({
        id: note.id,
        title: note.id === selectedId ? title : note.title,
      });
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleNoteMenuAction = (action: NoteContextMenuAction) => {
    const note = noteMenuTarget;
    if (!note) return;

    if (action === "export") {
      setNoteMenuClosing(true);
      void handleExportNote(note);
      return;
    }

    if (action === "move") {
      setNoteMenuMode("move");
      return;
    }

    setNoteMenuClosing(true);
    void handleDeleteNote(note.id);
  };

  const handleMoveNote = async (noteId: string, targetCategory: string) => {
    setNoteMenuClosing(true);
    setErrorMessage(null);
    try {
      await moveNoteCategory(noteId, targetCategory);
      await refreshNotes();
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleCreateCategory = async () => {
    const name = categoryInputValue.trim();
    if (!name) {
      setShowCategoryInput(false);
      return;
    }
    setErrorMessage(null);
    try {
      await createCategory(name);
      setCategories((prev) => [...prev, name].sort());
      setShowCategoryInput(false);
      setCategoryInputValue("");
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleRenameCategory = async (oldName: string) => {
    const newName = renameCategoryValue.trim();
    if (!newName || newName === oldName) {
      setRenamingCategory(null);
      return;
    }
    setErrorMessage(null);
    try {
      await renameCategory(oldName, newName);
      await refreshNotes();
      setRenamingCategory(null);
      setRenameCategoryValue("");
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleDeleteCategory = async (name: string) => {
    setErrorMessage(null);
    try {
      await deleteCategory(name);
      await refreshNotes();
      if (activeCategory === name) {
        setActiveCategory("");
      }
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const toggleCategoryCollapse = (category: string) => {
    setCollapsedCategories((prev) => {
      const next = new Set(prev);
      if (next.has(category)) {
        next.delete(category);
      } else {
        next.add(category);
      }
      return next;
    });
  };

  const markDirty = () => {
    if (selectedId) setSaveState("dirty");
  };

  const handleUndo = () => {
    if (!selectedId) return;
    const textarea = contentRef.current;
    if (runEditorUndo(textarea)) {
      setContent(textarea?.value ?? content);
      markDirty();
    }
  };

  const handleOpenNotepad = async () => {
    setErrorMessage(null);
    try {
      await openNotepadWindow();
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    void isCurrentWindowMaximized().then(setIsMaximized);
  }, []);

  useEffect(() => {
    if (!isResizingSidebar) return;

    document.body.style.userSelect = "none";
    document.body.style.cursor = "col-resize";

    const onMouseMove = (e: globalThis.MouseEvent) => {
      const newWidth = Math.min(Math.max(e.clientX, 180), 500);
      setSidebarWidth(newWidth);
    };
    const onMouseUp = () => setIsResizingSidebar(false);

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    return () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    };
  }, [isResizingSidebar]);

  useEffect(() => {
    if (!isResizingSplit) return;

    document.body.style.userSelect = "none";
    document.body.style.cursor = "col-resize";

    const onMouseMove = (e: globalThis.MouseEvent) => {
      const container = splitContainerRef.current;
      if (!container) return;
      const rect = container.getBoundingClientRect();
      const ratio = (e.clientX - rect.left) / rect.width;
      setSplitRatio(Math.min(Math.max(ratio, 0.2), 0.8));
    };
    const onMouseUp = () => setIsResizingSplit(false);

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    return () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.userSelect = "";
      document.body.style.cursor = "";
    };
  }, [isResizingSplit]);

  const handlePinEntry = async () => {
    if (!selectedId) return;
    const isPinned = pinnedTileIds.has(selectedId);
    if (!isPinned && saveState === "dirty") {
      await saveCurrentNote();
    }

    setErrorMessage(null);
    try {
      const pinned = await toggleTileWindow(selectedId);
      setPinnedTileIds((previous) => {
        return syncPinnedTileIds(previous, selectedId, pinned);
      });
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const selectedTilePinned = selectedId ? pinnedTileIds.has(selectedId) : false;

  const handleTitleBarDrag = (event: MouseEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest("button")) return;
    void startCurrentWindowDrag().catch(() => undefined);
  };

  const toggleMaximize = () => {
    void toggleMaximizeCurrentWindow().then(() => isCurrentWindowMaximized().then(setIsMaximized));
  };

  const handleTitleBarDoubleClick = (event: MouseEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest("button")) return;
    toggleMaximize();
  };

  const handleMinimize = () => {
    void minimizeCurrentWindow();
  };

  const handleMaximize = () => {
    toggleMaximize();
  };

  const handleClose = () => {
    void closeCurrentWindow();
  };

  return (
    <div className="w-full h-screen flex flex-col">
      <div className="relative noise-bg bg-cloud overflow-hidden flex flex-col flex-1">
        <BackgroundLayer config={settingsConfig} />
        <div
          className="relative z-10 flex items-center justify-between pl-5 pr-0 h-11 bg-paper/55 backdrop-blur-[1px] border-b border-paper-deep/30 shrink-0 select-none cursor-default"
          onMouseDown={handleTitleBarDrag}
          onDoubleClick={handleTitleBarDoubleClick}
        >
          <div className="flex items-center gap-3 min-w-0">
            <span className="text-[13px] font-display font-medium text-ink-soft tracking-wide">
              花笺
            </span>
            <span className="text-[11px] text-ink-ghost font-body">—</span>
            <span className="text-[11px] text-ink-faint font-body truncate max-w-[240px]">
              {title ||
                selectedNote?.preview ||
                t("common.untitledNote", { defaultValue: "无标题笔记" })}
            </span>
          </div>
          <div className="flex items-center">
            {errorMessage && (
              <span className="max-w-[200px] truncate text-[11px] text-red-400 mr-2">
                {errorMessage}
              </span>
            )}
            <button
              onClick={() => void handleOpenNotepad()}
              className="w-10 h-11 flex items-center justify-center text-ink-ghost hover:text-bamboo hover:bg-bamboo-mist/50 transition-all cursor-pointer"
              title={t("main.window.quickNotepad", { defaultValue: "快捷便签" })}
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M4 4h16v14H7l-3 3V4z" />
                <path d="M8 9h8M8 13h5" />
              </svg>
            </button>
            <button
              onClick={() => void handleOpenSettings()}
              className="w-10 h-11 flex items-center justify-center text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer"
              title={t("main.window.settings", { defaultValue: "设置" })}
            >
              <svg
                width="14"
                height="14"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>

            <div className="w-px h-4 bg-paper-deep/30 mx-0.5" />

            <button
              onClick={handleMinimize}
              className="w-11 h-11 flex items-center justify-center text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-all cursor-pointer"
              title={t("main.window.minimize", { defaultValue: "最小化" })}
            >
              <svg width="12" height="12" viewBox="0 0 12 12">
                <rect x="1" y="5.5" width="10" height="1" fill="currentColor" rx="0.5" />
              </svg>
            </button>
            <button
              onClick={handleMaximize}
              className="w-11 h-11 flex items-center justify-center text-ink-ghost hover:text-ink-soft hover:bg-paper-warm transition-all cursor-pointer"
              title={
                isMaximized
                  ? t("main.window.restore", { defaultValue: "还原" })
                  : t("main.window.maximize", { defaultValue: "最大化" })
              }
            >
              {isMaximized ? (
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 12 12"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.2"
                >
                  <rect x="3" y="3" width="7" height="7" rx="1" />
                  <path d="M3 5H2V2a1 1 0 0 1 1-1h5v1" />
                </svg>
              ) : (
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 12 12"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.2"
                >
                  <rect x="1.5" y="1.5" width="9" height="9" rx="1.5" />
                </svg>
              )}
            </button>
            <button
              onClick={handleClose}
              className="w-11 h-11 flex items-center justify-center text-ink-ghost hover:text-red-500 hover:bg-danger-bg transition-all cursor-pointer"
              title={t("main.window.close", { defaultValue: "关闭" })}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 12 12"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              >
                <path d="M2 2l8 8M10 2l-8 8" />
              </svg>
            </button>
          </div>
        </div>

        <div className="relative z-10 flex flex-1 min-h-0">
          <div
            className={`border-r border-paper-deep/30 bg-paper/40 flex flex-col shrink-0 ${
              sidebarCollapsed ? "w-0 overflow-hidden transition-all duration-[600ms]" : ""
            }`}
            style={sidebarCollapsed ? undefined : { width: `${sidebarWidth}px` }}
          >
            <div className="px-3 pt-3 pb-2 shrink-0">
              <div className="flex items-center gap-2 px-2.5 h-8 rounded-lg bg-paper-warm/80 border border-paper-deep/40 focus-within:border-bamboo/30 focus-within:bg-cloud transition-all">
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  className="text-ink-ghost shrink-0"
                >
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.35-4.35" />
                </svg>
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                  placeholder={t("main.sidebar.searchPlaceholder", { defaultValue: "搜索笔记…" })}
                  className="flex-1 text-[12px] font-body text-ink placeholder:text-ink-ghost/60 bg-transparent"
                />
                {searchQuery && (
                  <button
                    onClick={() => setSearchQuery("")}
                    className="text-ink-ghost hover:text-ink-faint transition-colors cursor-pointer"
                    title={t("main.sidebar.clearSearch", { defaultValue: "清空搜索" })}
                  >
                    <svg
                      width="10"
                      height="10"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="3"
                      strokeLinecap="round"
                    >
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                )}
              </div>
            </div>

            <div className="px-3 pb-2 shrink-0 space-y-1">
              <button
                onClick={handleNewNote}
                className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[12px] font-body text-bamboo hover:bg-bamboo-mist/60 transition-all cursor-pointer group"
              >
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  className="group-hover:rotate-90 transition-transform duration-200"
                >
                  <path d="M12 5v14M5 12h14" />
                </svg>
                <span>{t("main.sidebar.newNote", { defaultValue: "新建笔记" })}</span>
              </button>
              <button
                onClick={() => void handleImportNote()}
                className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[12px] font-body text-ink-faint hover:text-bamboo hover:bg-bamboo-mist/50 transition-all cursor-pointer group"
              >
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <path d="M12 3v12" />
                  <path d="m7 10 5 5 5-5" />
                  <path d="M5 21h14" />
                </svg>
                <span>{t("main.sidebar.importMarkdown", { defaultValue: "导入 Markdown" })}</span>
              </button>
            </div>

            <div className="flex items-center justify-between px-5 pb-1.5 shrink-0">
              <span className="text-[10px] text-ink-ghost font-mono tracking-wider uppercase">
                {t("common.noteCount", {
                  count: filteredNotes.length,
                  defaultValue: "{{count}} 篇笔记",
                })}
                {externalFiles.length > 0
                  ? ` · ${t("common.externalFileCount", {
                      count: externalFiles.length,
                      defaultValue: "{{count}} 个外部文件",
                    })}`
                  : ""}
              </span>
              <button
                onClick={() => setShowCategoryInput(true)}
                className="text-[10px] text-ink-ghost hover:text-bamboo transition-colors cursor-pointer"
                title={t("main.category.new", { defaultValue: "新建分类" })}
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                >
                  <path d="M12 5v14M5 12h14" />
                </svg>
              </button>
            </div>

            {showCategoryInput && (
              <div className="px-3 pb-2 shrink-0">
                <input
                  type="text"
                  autoFocus
                  value={categoryInputValue}
                  onChange={(e) => setCategoryInputValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void handleCreateCategory();
                    if (e.key === "Escape") {
                      setShowCategoryInput(false);
                      setCategoryInputValue("");
                    }
                  }}
                  onBlur={() => void handleCreateCategory()}
                  placeholder={t("main.category.placeholder", { defaultValue: "输入分类名…" })}
                  className="w-full px-2.5 h-7 rounded-lg text-[12px] font-body text-ink bg-paper-warm/80 border border-paper-deep/40 focus:border-bamboo/30 placeholder:text-ink-ghost/60"
                />
              </div>
            )}

            <div className="flex-1 overflow-y-auto px-2 pb-2">
              <div className="space-y-0.5">
                {externalFiles.length > 0 && (
                  <>
                    <div className="px-3 py-1.5 text-[10px] text-ink-ghost/50 font-mono tracking-wider uppercase">
                      {t("main.externalFiles.title", { defaultValue: "外部文件" })}
                    </div>
                    {externalFiles.map((file) => {
                      const isSelected = file.id === selectedId;
                      const isHovered = file.id === hoveredId;

                      return (
                        <button
                          key={file.id}
                          onClick={() => void handleSelectExternalFile(file.id)}
                          onMouseEnter={() => setHoveredId(file.id)}
                          onMouseLeave={() => setHoveredId(null)}
                          className={`w-full text-left rounded-xl px-3 py-2.5 transition-all duration-[600ms] cursor-pointer group relative ${
                            isSelected
                              ? "bg-bamboo-mist/70"
                              : isHovered
                                ? "bg-paper-warm/70"
                                : "bg-transparent"
                          }`}
                        >
                          <div
                            className={`absolute left-0 top-1/2 -translate-y-1/2 w-[3px] rounded-r-full bg-bamboo/60 transition-all duration-[600ms] ${
                              isSelected ? "h-5 opacity-100" : "h-0 opacity-0"
                            }`}
                          />

                          <div className="flex items-baseline justify-between mb-0.5">
                            <span
                              className={`text-[13px] font-display font-medium truncate pr-2 transition-colors flex items-center gap-1.5 ${
                                isSelected ? "text-bamboo" : "text-ink-soft"
                              }`}
                            >
                              <svg
                                width="12"
                                height="12"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                strokeWidth="2"
                                strokeLinecap="round"
                                strokeLinejoin="round"
                                className="shrink-0 opacity-60"
                              >
                                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                                <polyline points="14 2 14 8 20 8" />
                              </svg>
                              {file.title}
                            </span>
                            <button
                              onClick={(e) => {
                                e.stopPropagation();
                                handleRemoveExternalFile(file.id);
                              }}
                              className="opacity-0 group-hover:opacity-100 text-ink-ghost hover:text-red-400 transition-all p-0.5"
                              title={t("main.externalFiles.remove", { defaultValue: "从列表移除" })}
                            >
                              <svg
                                width="12"
                                height="12"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                strokeWidth="2"
                                strokeLinecap="round"
                              >
                                <line x1="18" y1="6" x2="6" y2="18" />
                                <line x1="6" y1="6" x2="18" y2="18" />
                              </svg>
                            </button>
                          </div>

                          <p className="text-[11px] text-ink-ghost leading-relaxed line-clamp-2 group-hover:text-ink-faint transition-colors pl-[18px]">
                            {file.filePath}
                          </p>
                        </button>
                      );
                    })}
                  </>
                )}

                {categoryGroups.map((group: CategoryGroup) => {
                  if (!group.category) {
                    return (
                      <div
                        key="__uncategorized__"
                        className={`rounded-lg transition-all duration-200 ${
                          dragOverCategory === "" ? "bg-bamboo/10 ring-1 ring-bamboo/20" : ""
                        }`}
                        onDragOver={(e) => {
                          e.preventDefault();
                          e.dataTransfer.dropEffect = "move";
                          setDragOverCategory("");
                        }}
                        onDragLeave={(e) => {
                          if (!e.currentTarget.contains(e.relatedTarget as Node)) {
                            setDragOverCategory(null);
                          }
                        }}
                        onDrop={(e) => {
                          e.preventDefault();
                          setDragOverCategory(null);
                          const noteId = e.dataTransfer.getData("text/plain");
                          if (noteId) void handleMoveNote(noteId, "");
                        }}
                      >
                        {group.notes.map((note) => {
                          const isSelected = note.id === selectedId;
                          const isHovered = note.id === hoveredId;
                          return (
                            <div
                              key={note.id}
                              draggable
                              onDragStart={(e) => {
                                e.dataTransfer.setData("text/plain", note.id);
                                e.dataTransfer.effectAllowed = "move";
                              }}
                              onClick={() => void handleSelectNote(note.id)}
                              onContextMenu={(event) => handleOpenNoteMenu(event, note.id)}
                              onMouseEnter={() => setHoveredId(note.id)}
                              onMouseLeave={() => setHoveredId(null)}
                              className={`w-full text-left rounded-xl px-3 py-2.5 transition-all duration-[600ms] cursor-pointer group relative ${
                                isSelected
                                  ? "bg-bamboo-mist/70"
                                  : isHovered
                                    ? "bg-paper-warm/70"
                                    : "bg-transparent"
                              }`}
                            >
                              <div
                                className={`absolute left-0 top-1/2 -translate-y-1/2 w-[3px] rounded-r-full bg-bamboo/60 transition-all duration-[600ms] ${
                                  isSelected ? "h-5 opacity-100" : "h-0 opacity-0"
                                }`}
                              />
                              <div className="flex items-baseline justify-between mb-0.5">
                                <span
                                  className={`text-[13px] font-display font-medium truncate pr-2 transition-colors ${
                                    isSelected ? "text-bamboo" : "text-ink-soft"
                                  }`}
                                >
                                  {getDisplayTitle(note, t)}
                                </span>
                                <span className="text-[10px] text-ink-ghost font-mono tabular-nums shrink-0">
                                  {formatShortDate(note.updatedAt)}
                                </span>
                              </div>
                              <p className="text-[11px] text-ink-ghost leading-relaxed line-clamp-2 group-hover:text-ink-faint transition-colors">
                                {note.preview ||
                                  t("common.blankNote", { defaultValue: "空白笔记" })}
                              </p>
                              <div className="flex items-center gap-2 mt-1">
                                <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                                  {formatTime(note.updatedAt)}
                                </span>
                                <span className="text-[10px] text-ink-ghost/40">·</span>
                                <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                                  {t("common.wordCount", {
                                    count: note.wordCount,
                                    defaultValue: "{{count}} 字",
                                  })}
                                </span>
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    );
                  }

                  const isCollapsed = collapsedCategories.has(group.category);

                  return (
                    <div key={group.category} className="px-2 mb-0.5">
                      <div
                        className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg group/cat cursor-pointer select-none transition-all duration-200 ${
                          dragOverCategory === group.category
                            ? "bg-bamboo/15 border border-bamboo/40 ring-1 ring-bamboo/20"
                            : isCollapsed
                              ? "bg-transparent border border-bamboo/15"
                              : "bg-bamboo/8 border border-bamboo/15 rounded-b-none"
                        }`}
                        onClick={() => toggleCategoryCollapse(group.category)}
                        onContextMenu={(e) => {
                          e.preventDefault();
                          e.stopPropagation();
                          setCategoryMenu({ x: e.clientX, y: e.clientY, category: group.category });
                          setCategoryMenuClosing(false);
                          setCategoryMenuConfirmDelete(false);
                        }}
                        onDragOver={(e) => {
                          e.preventDefault();
                          e.dataTransfer.dropEffect = "move";
                          setDragOverCategory(group.category);
                        }}
                        onDragLeave={() => setDragOverCategory(null)}
                        onDrop={(e) => {
                          e.preventDefault();
                          setDragOverCategory(null);
                          const noteId = e.dataTransfer.getData("text/plain");
                          if (noteId) void handleMoveNote(noteId, group.category);
                        }}
                      >
                        <svg
                          width="10"
                          height="10"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2.5"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          className={`text-bamboo/50 shrink-0 transition-transform duration-200 ${isCollapsed ? "" : "rotate-90"}`}
                        >
                          <polyline points="9 18 15 12 9 6" />
                        </svg>
                        <svg
                          width="12"
                          height="12"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          strokeWidth="2"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          className="text-bamboo/50 shrink-0"
                        >
                          <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
                        </svg>
                        {renamingCategory === group.category ? (
                          <input
                            type="text"
                            autoFocus
                            value={renameCategoryValue}
                            onChange={(e) => setRenameCategoryValue(e.target.value)}
                            onKeyDown={(e) => {
                              e.stopPropagation();
                              if (e.key === "Enter") void handleRenameCategory(group.category);
                              if (e.key === "Escape") setRenamingCategory(null);
                            }}
                            onBlur={() => void handleRenameCategory(group.category)}
                            onClick={(e) => e.stopPropagation()}
                            className="flex-1 min-w-0 px-1 text-[10px] font-mono text-ink bg-paper-warm/80 border border-bamboo/30 rounded"
                          />
                        ) : (
                          <span className="text-[11px] text-bamboo/70 font-medium truncate">
                            {group.category}
                          </span>
                        )}
                        <span className="text-[9px] text-bamboo/40 font-mono ml-auto shrink-0">
                          {group.notes.length}
                        </span>
                      </div>

                      <div className={`category-body ${isCollapsed ? "" : "expanded"}`}>
                        <div
                          className="category-body-inner bg-bamboo/[0.03] border border-t-0 border-bamboo/10 rounded-b-lg pb-1 pt-1"
                          onDragOver={(e) => {
                            e.preventDefault();
                            e.dataTransfer.dropEffect = "move";
                            setDragOverCategory(group.category);
                          }}
                          onDragLeave={(e) => {
                            if (!e.currentTarget.contains(e.relatedTarget as Node)) {
                              setDragOverCategory(null);
                            }
                          }}
                          onDrop={(e) => {
                            e.preventDefault();
                            setDragOverCategory(null);
                            const noteId = e.dataTransfer.getData("text/plain");
                            if (noteId) void handleMoveNote(noteId, group.category);
                          }}
                        >
                          {group.notes.length === 0 ? (
                            <div className="px-3 py-3 text-center text-[11px] text-ink-ghost/50">
                              {t("main.category.emptyFolder", { defaultValue: "空文件夹" })}
                            </div>
                          ) : (
                            group.notes.map((note) => {
                              const isSelected = note.id === selectedId;
                              const isHovered = note.id === hoveredId;

                              return (
                                <div
                                  key={note.id}
                                  draggable
                                  onDragStart={(e) => {
                                    e.dataTransfer.setData("text/plain", note.id);
                                    e.dataTransfer.effectAllowed = "move";
                                  }}
                                  onClick={() => void handleSelectNote(note.id)}
                                  onContextMenu={(event) => handleOpenNoteMenu(event, note.id)}
                                  onMouseEnter={() => setHoveredId(note.id)}
                                  onMouseLeave={() => setHoveredId(null)}
                                  className={`w-full text-left rounded-lg mx-1 px-2.5 py-2 transition-all duration-[600ms] cursor-pointer group relative ${
                                    isSelected
                                      ? "bg-bamboo-mist/70"
                                      : isHovered
                                        ? "bg-paper-warm/70"
                                        : "bg-transparent"
                                  }`}
                                  style={{ width: "calc(100% - 8px)" }}
                                >
                                  <div
                                    className={`absolute left-0 top-1/2 -translate-y-1/2 w-[3px] rounded-r-full bg-bamboo/60 transition-all duration-[600ms] ${
                                      isSelected ? "h-5 opacity-100" : "h-0 opacity-0"
                                    }`}
                                  />

                                  <div className="flex items-baseline justify-between mb-0.5">
                                    <span
                                      className={`text-[13px] font-display font-medium truncate pr-2 transition-colors ${
                                        isSelected ? "text-bamboo" : "text-ink-soft"
                                      }`}
                                    >
                                      {getDisplayTitle(note, t)}
                                    </span>
                                    <span className="text-[10px] text-ink-ghost font-mono tabular-nums shrink-0">
                                      {formatShortDate(note.updatedAt)}
                                    </span>
                                  </div>

                                  <p className="text-[11px] text-ink-ghost leading-relaxed line-clamp-2 group-hover:text-ink-faint transition-colors">
                                    {note.preview ||
                                      t("common.blankNote", { defaultValue: "空白笔记" })}
                                  </p>

                                  <div className="flex items-center gap-2 mt-1">
                                    <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                                      {formatTime(note.updatedAt)}
                                    </span>
                                    <span className="text-[10px] text-ink-ghost/40">·</span>
                                    <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                                      {t("common.wordCount", {
                                        count: note.wordCount,
                                        defaultValue: "{{count}} 字",
                                      })}
                                    </span>
                                  </div>
                                </div>
                              );
                            })
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}

                {!isLoading && filteredNotes.length === 0 && externalFiles.length === 0 && (
                  <div className="px-3 py-8 text-center text-[12px] text-ink-ghost leading-relaxed">
                    {searchQuery
                      ? t("main.search.noResults", { defaultValue: "没有匹配的笔记" })
                      : t("main.search.empty", { defaultValue: "还没有笔记" })}
                  </div>
                )}
              </div>
            </div>
          </div>

          {!sidebarCollapsed && (
            <div
              className={`w-1 shrink-0 cursor-col-resize group relative ${isResizingSidebar ? "bg-bamboo/30" : "hover:bg-bamboo/20"} transition-colors`}
              onMouseDown={(e) => {
                e.preventDefault();
                setIsResizingSidebar(true);
              }}
            >
              <div
                className={`absolute inset-y-0 -left-1 -right-1 ${isResizingSidebar ? "" : "group-hover:bg-bamboo/5"}`}
              />
            </div>
          )}

          <div className="flex-1 flex flex-col min-w-0">
            <div className="flex items-center justify-between px-4 h-10 border-b border-paper-deep/20 shrink-0 bg-paper/20">
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer"
                  title={
                    sidebarCollapsed
                      ? t("main.window.expandSidebar", { defaultValue: "展开侧栏" })
                      : t("main.window.collapseSidebar", { defaultValue: "收起侧栏" })
                  }
                >
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                    <line x1="9" y1="3" x2="9" y2="21" />
                  </svg>
                </button>

                <div className="h-4 w-px bg-paper-deep/30 mx-1" />

                <button
                  onClick={() => void handlePinEntry()}
                  disabled={!selectedId}
                  aria-label={pinTileButtonTitle(selectedTilePinned)}
                  className={`w-7 h-7 flex items-center justify-center rounded-lg transition-all cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed ${
                    selectedTilePinned
                      ? "text-bamboo bg-bamboo-mist/40 hover:text-red-400 hover:bg-danger-bg"
                      : "text-ink-ghost hover:text-bamboo hover:bg-bamboo-mist/50"
                  }`}
                  title={pinTileButtonTitle(selectedTilePinned)}
                >
                  <svg
                    width="13"
                    height="13"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M12 17v5" />
                    <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v1a1 1 0 0 0 1 1 1 1 0 0 1 1 1z" />
                  </svg>
                </button>

                <button
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={handleUndo}
                  disabled={!selectedId}
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                  title={t("main.editor.undo", { defaultValue: "撤销（Ctrl+Z）" })}
                  aria-label={t("main.editor.undoLabel", { defaultValue: "撤销" })}
                >
                  <svg
                    data-testid="main-editor-undo-icon"
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <path d="M9 14 4 9l5-5" />
                    <path d="M4 9h10a6 6 0 0 1 0 12h-1" />
                  </svg>
                </button>

                <button
                  onClick={() => void saveCurrentNote()}
                  disabled={!selectedId || saveState === "saving"}
                  className="px-2.5 h-7 flex items-center justify-center rounded-lg text-[11px] text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                  title={t("common.save", { defaultValue: "保存" })}
                >
                  {t("common.save", { defaultValue: "保存" })}
                </button>

                {deleteConfirm ? (
                  <div
                    className={`flex items-center gap-1 ml-1 ${deleteExiting ? "animate-delete-confirm-exit" : "animate-delete-confirm"}`}
                  >
                    <span className="text-[11px] text-red-400 whitespace-nowrap">
                      {t("main.editor.confirmDelete", { defaultValue: "确认删除？" })}
                    </span>
                    <button
                      onClick={() => {
                        setDeleteExiting(true);
                        setTimeout(() => {
                          setDeleteExiting(false);
                          setDeleteConfirm(false);
                          void handleDeleteNote();
                        }, 150);
                      }}
                      className="px-2 h-6 rounded-md text-[11px] text-cloud bg-red-400 hover:bg-red-500 transition-colors cursor-pointer whitespace-nowrap"
                    >
                      {t("common.delete", { defaultValue: "删除" })}
                    </button>
                    <button
                      onClick={() => {
                        setDeleteExiting(true);
                        setTimeout(() => {
                          setDeleteExiting(false);
                          setDeleteConfirm(false);
                        }, 150);
                      }}
                      className="px-2 h-6 rounded-md text-[11px] text-ink-faint hover:text-ink-soft hover:bg-paper-warm transition-colors cursor-pointer"
                    >
                      {t("common.cancel", { defaultValue: "取消" })}
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => setDeleteConfirm(true)}
                    disabled={!selectedId}
                    className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-red-400 hover:bg-danger-bg transition-all cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                    title={t("noteMenu.delete", { defaultValue: "删除笔记" })}
                  >
                    <svg
                      width="13"
                      height="13"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    >
                      <polyline points="3,6 5,6 21,6" />
                      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                    </svg>
                  </button>
                )}
              </div>

              <SlidingButtonGroup
                options={viewModeOptions}
                value={viewMode}
                onChange={setViewMode}
                buttonClassName="px-3 py-1"
              />
            </div>

            <div
              key={noteTransitionKey}
              className="animate-note-enter px-6 pt-4 pb-2 shrink-0 border-b border-paper-deep/15"
            >
              <input
                type="text"
                value={title}
                onChange={(event) => {
                  setTitle(event.target.value);
                  markDirty();
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    contentRef.current?.focus();
                  }
                }}
                placeholder={t("common.untitledNote", { defaultValue: "无标题笔记" })}
                disabled={!selectedId}
                className="w-full text-[20px] font-display font-bold text-ink placeholder:text-ink-ghost/50 tracking-wide disabled:opacity-60"
              />
              <div className="flex items-center gap-3 mt-1.5">
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums truncate max-w-[200px]">
                  {selectedExternalFile
                    ? t("main.externalFile.label", {
                        path: selectedExternalFile.filePath,
                        defaultValue: "外部文件 · {{path}}",
                      })
                    : selectedNote
                      ? `${formatShortDate(selectedNote.updatedAt)} ${formatTime(selectedNote.updatedAt)}`
                      : "--"}
                </span>
                <span className="text-[10px] text-ink-ghost/40">·</span>
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {t("common.wordCount", { count: charCount, defaultValue: "{{count}} 字" })}
                </span>
                <span className="text-[10px] text-ink-ghost/40">·</span>
                <span
                  key={saveState}
                  className={`text-[10px] font-mono tabular-nums animate-status-fade ${
                    saveState === "error"
                      ? "text-red-400"
                      : saveState === "dirty"
                        ? "text-amber-500/70"
                        : "text-bamboo/60"
                  }`}
                >
                  {saveStateLabel[saveState]}
                </span>
              </div>
            </div>

            <div
              key={viewMode}
              ref={splitContainerRef}
              className="flex-1 flex min-h-0 animate-view-fade"
            >
              {!selectedId && !isLoading ? (
                <div className="flex-1 flex items-center justify-center text-[13px] text-ink-ghost">
                  {t("main.editor.emptyHint", { defaultValue: "选择或新建一篇笔记" })}
                </div>
              ) : (
                <>
                  {(viewMode === "edit" || viewMode === "split") && (
                    <div
                      className="flex flex-col min-h-0 shrink-0"
                      style={{ width: viewMode === "split" ? `${splitRatio * 100}%` : "100%" }}
                    >
                      <div className="flex items-center gap-0.5 px-4 pt-2 pb-1 shrink-0">
                        {toolbarButtons.map((button) => (
                          <button
                            key={button.label}
                            title={button.title}
                            onMouseDown={(e) => e.preventDefault()}
                            onClick={() => {
                              if (contentRef.current) {
                                applyFormat(
                                  contentRef.current,
                                  button.action,
                                  t,
                                  setContent,
                                  markDirty,
                                );
                              }
                            }}
                            className={`w-6 h-6 flex items-center justify-center rounded text-[11px] text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer ${button.style}`}
                          >
                            {button.label}
                          </button>
                        ))}
                      </div>

                      <div className="flex-1 overflow-hidden px-5 pb-4">
                        <textarea
                          ref={contentRef}
                          data-tab-indent="true"
                          value={content}
                          onChange={(event) => {
                            setContent(event.target.value);
                            markDirty();
                          }}
                          className="w-full h-full leading-[1.9] text-ink-soft font-body placeholder:text-ink-ghost/40"
                          style={{
                            fontSize: `${settingsConfig?.fontSize ?? 14}px`,
                            tabSize: `var(--tab-indent-size, 2)`,
                          }}
                          placeholder={t("main.editor.contentPlaceholder", {
                            defaultValue: "开始写作……",
                          })}
                          spellCheck={false}
                          disabled={!selectedId}
                        />
                      </div>
                    </div>
                  )}

                  {viewMode === "split" && (
                    <div
                      className={`w-1.5 shrink-0 cursor-col-resize group relative flex items-center justify-center ${isResizingSplit ? "bg-bamboo/30" : "hover:bg-bamboo/20"} transition-colors`}
                      onMouseDown={(e) => {
                        e.preventDefault();
                        setIsResizingSplit(true);
                      }}
                    >
                      <div
                        className={`absolute inset-y-0 -left-1.5 -right-1.5 ${isResizingSplit ? "" : "group-hover:bg-bamboo/5"}`}
                      />
                      {/* 拖拽手柄指示器 */}
                      <div className="relative z-10 flex flex-col gap-[3px] opacity-0 group-hover:opacity-100 transition-opacity">
                        <div className="w-[3px] h-[3px] rounded-full bg-ink-ghost/60" />
                        <div className="w-[3px] h-[3px] rounded-full bg-ink-ghost/60" />
                        <div className="w-[3px] h-[3px] rounded-full bg-ink-ghost/60" />
                      </div>
                    </div>
                  )}

                  {(viewMode === "preview" || viewMode === "split") && (
                    <div className="flex flex-col min-h-0 min-w-0 flex-1">
                      {viewMode === "split" && (
                        <div className="px-4 pt-2.5 pb-1 shrink-0">
                          <span className="text-[10px] text-ink-ghost/60 font-mono tracking-widest uppercase">
                            {t("main.editor.previewLabel", { defaultValue: "Preview" })}
                          </span>
                        </div>
                      )}
                      <div
                        className={`flex-1 overflow-y-auto px-6 pb-6 ${
                          viewMode === "preview" ? "pt-3" : "pt-1"
                        }`}
                      >
                        <MarkdownPreview
                          content={content}
                          fontSize={settingsConfig?.fontSize ?? 14}
                          renderHtml={settingsConfig?.renderHtmlMarkdown ?? false}
                        />
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>

            <div className="flex items-center justify-between px-4 h-7 border-t border-paper-deep/20 bg-paper/30 shrink-0">
              <div className="flex items-center gap-3">
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {t("main.statusBar.lineNumber", {
                    count: lineCount,
                    defaultValue: "Ln {{count}}",
                  })}
                </span>
                <span className="text-[10px] text-ink-ghost/40">|</span>
                <span className="text-[10px] text-ink-ghost font-mono">
                  {t("main.statusBar.format", { defaultValue: "Markdown + LaTeX" })}
                </span>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-[10px] text-ink-ghost font-mono">
                  {t("main.statusBar.encoding", { defaultValue: "UTF-8" })}
                </span>
                <span className="text-[10px] text-ink-ghost/40">|</span>
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {t("main.statusBar.byteSize", { size: byteSize, defaultValue: "{{size}} KB" })}
                </span>
              </div>
            </div>
          </div>
          {settingsConfig && settingsOpen && settingsOverlay && (
            <div className="absolute inset-0 z-20" onClick={handleCloseSettings} />
          )}
          {settingsConfig && (
            <div
              className={`transition-all duration-[600ms] overflow-hidden h-full ${
                settingsOverlay
                  ? `absolute right-0 top-0 bottom-0 z-30 ${settingsOpen ? "w-[360px] shadow-xl" : "w-0"}`
                  : `relative shrink-0 ${settingsOpen ? "w-[360px]" : "w-0"}`
              }`}
            >
              <div className="w-[360px] h-full">
                <SettingsPanel
                  config={settingsConfig}
                  onChange={handleSettingsChange}
                  onChooseNotesDir={() => void handleChooseNotesDir()}
                  onClose={handleCloseSettings}
                />
              </div>
            </div>
          )}
        </div>
      </div>
      {noteMenu && noteMenuTarget && (
        <div
          className={`fixed z-[9999] min-w-[168px] py-1.5 bg-cloud/95 backdrop-blur-sm border border-paper-deep/50 rounded-lg overflow-hidden select-none ${noteMenuClosing ? "animate-menu-exit" : "animate-menu-enter"}`}
          style={{ left: noteMenu.x, top: noteMenu.y }}
          onMouseDown={(event) => event.stopPropagation()}
        >
          {noteMenuMode === "main" ? (
            <div key="main" className="animate-menu-slide-right">
              {noteContextMenuItems.map((item, index) => (
                <button
                  key={item.action}
                  onClick={() => handleNoteMenuAction(item.action)}
                  className={`w-full flex items-center justify-between px-3 py-1.5 text-[12px] font-body transition-colors cursor-pointer ${
                    item.tone === "danger"
                      ? "text-red-400 hover:bg-danger-bg hover:text-red-500"
                      : "text-ink-soft hover:bg-bamboo-mist/60 hover:text-bamboo"
                  } ${index > 0 ? "border-t border-paper-deep/20" : ""}`}
                >
                  <span>{item.label}</span>
                </button>
              ))}
            </div>
          ) : (
            <div key="move" className="animate-menu-slide-left">
              <button
                onClick={() => setNoteMenuMode("main")}
                className="w-full flex items-center gap-1.5 px-3 py-1.5 text-[12px] font-body text-ink-ghost hover:bg-paper-warm transition-colors cursor-pointer border-b border-paper-deep/20"
              >
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  <polyline points="15 18 9 12 15 6" />
                </svg>
                <span>{t("common.back", { defaultValue: "返回" })}</span>
              </button>
              <button
                onClick={() => void handleMoveNote(noteMenuTarget.id, "")}
                className="w-full text-left px-3 py-1.5 text-[12px] font-body text-ink-soft hover:bg-bamboo-mist/60 hover:text-bamboo transition-colors cursor-pointer"
              >
                {t("main.category.uncategorized", { defaultValue: "未分类" })}
              </button>
              {categories.map((cat) => (
                <button
                  key={cat}
                  onClick={() => void handleMoveNote(noteMenuTarget.id, cat)}
                  className="w-full text-left px-3 py-1.5 text-[12px] font-body text-ink-soft hover:bg-bamboo-mist/60 hover:text-bamboo transition-colors cursor-pointer"
                >
                  {cat}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {categoryMenu && (
        <div
          className={`fixed z-[9999] min-w-[140px] py-1.5 bg-cloud/95 backdrop-blur-sm border border-paper-deep/50 rounded-lg overflow-hidden select-none ${categoryMenuClosing ? "animate-menu-exit" : "animate-menu-enter"}`}
          style={{ left: categoryMenu.x, top: categoryMenu.y }}
          onMouseDown={(event) => event.stopPropagation()}
        >
          {categoryMenuConfirmDelete ? (
            <div className="animate-menu-slide-left">
              <div className="px-3 py-1.5 text-[11px] font-body text-ink-faint border-b border-paper-deep/20">
                {t("main.category.confirmDelete", {
                  category: categoryMenu.category,
                  defaultValue: "确认删除「{{category}}」？",
                })}
              </div>
              <button
                onClick={() => {
                  void handleDeleteCategory(categoryMenu.category);
                  setCategoryMenuClosing(true);
                }}
                className="w-full text-left px-3 py-1.5 text-[12px] font-body text-red-400 hover:bg-danger-bg hover:text-red-500 transition-colors cursor-pointer"
              >
                {t("main.category.confirmDeleteAction", { defaultValue: "确认删除" })}
              </button>
              <button
                onClick={() => setCategoryMenuConfirmDelete(false)}
                className="w-full text-left px-3 py-1.5 text-[12px] font-body text-ink-soft hover:bg-bamboo-mist/60 hover:text-bamboo transition-colors cursor-pointer"
              >
                {t("common.cancel", { defaultValue: "取消" })}
              </button>
            </div>
          ) : (
            <div className="animate-menu-slide-right">
              <button
                onClick={() => {
                  setCategoryMenuClosing(true);
                  setRenamingCategory(categoryMenu.category);
                  setRenameCategoryValue(categoryMenu.category);
                }}
                className="w-full text-left px-3 py-1.5 text-[12px] font-body text-ink-soft hover:bg-bamboo-mist/60 hover:text-bamboo transition-colors cursor-pointer"
              >
                {t("main.category.rename", { defaultValue: "重命名" })}
              </button>
              <button
                onClick={() => setCategoryMenuConfirmDelete(true)}
                className="w-full text-left px-3 py-1.5 text-[12px] font-body text-red-400 hover:bg-danger-bg hover:text-red-500 transition-colors cursor-pointer border-t border-paper-deep/20"
              >
                {t("main.category.delete", { defaultValue: "删除分类" })}
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
