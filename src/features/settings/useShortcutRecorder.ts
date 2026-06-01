import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface UseShortcutRecorderOptions {
  onRecord: (shortcut: string) => void;
}

export interface ShortcutRecorderHandle {
  isRecording: boolean;
  heldKeys: string[];
  startRecording: () => void;
  cancelRecording: () => void;
}

interface HookKeyEvent {
  key: string;
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
}

const MODIFIER_EVENT_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

const CODE_TO_KEY: Record<string, string> = {
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
  Minus: "-",
  Equal: "=",
};

function normalizeKey(key: string, code?: string): string {
  if (key === " ") return "Space";
  if (key.length === 1 && key.charCodeAt(0) < 0x20 && code) {
    return CODE_TO_KEY[code] ?? code;
  }
  if (key.length === 1) return key.toUpperCase();
  return key;
}

function buildShortcutString(
  ctrl: boolean,
  alt: boolean,
  shift: boolean,
  meta: boolean,
  key: string,
): string {
  const parts: string[] = [];
  if (ctrl) parts.push("Control");
  if (alt) parts.push("Alt");
  if (shift) parts.push("Shift");
  if (meta) parts.push("Meta");
  parts.push(key);
  return parts.join("+");
}

export function useShortcutRecorder({
  onRecord,
}: UseShortcutRecorderOptions): ShortcutRecorderHandle {
  const onRecordRef = useRef(onRecord);
  onRecordRef.current = onRecord;
  const [isRecording, setIsRecording] = useState(false);
  const [heldKeys, setHeldKeys] = useState<string[]>([]);

  const finishRecording = useCallback((shortcut: string) => {
    setIsRecording(false);
    setHeldKeys([]);
    onRecordRef.current(shortcut);
  }, []);

  const cancelRecording = useCallback(() => {
    setIsRecording(false);
    setHeldKeys([]);
  }, []);

  // DOM keydown handler — fallback when the system hook hasn't started yet
  useEffect(() => {
    if (!isRecording) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (MODIFIER_EVENT_KEYS.has(e.key)) return;

      if (e.key === "Escape") {
        e.preventDefault();
        cancelRecording();
        return;
      }

      if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        finishRecording("");
        return;
      }

      e.preventDefault();
      finishRecording(
        buildShortcutString(
          e.ctrlKey,
          e.altKey,
          e.shiftKey,
          e.metaKey,
          normalizeKey(e.key, e.code),
        ),
      );
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [isRecording, finishRecording, cancelRecording]);

  // System keyboard hook — unregisters app shortcuts & intercepts system shortcuts
  useEffect(() => {
    if (!isRecording) return;

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    listen<HookKeyEvent>("shortcut-hook-key", (event) => {
      if (cancelled) return;
      const { key, ctrl, alt, shift, meta } = event.payload;

      if (key === "Escape") {
        cancelRecording();
        return;
      }

      if (key === "Delete" || key === "Backspace") {
        finishRecording("");
        return;
      }

      finishRecording(buildShortcutString(ctrl, alt, shift, meta, key));
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
        invoke("start_shortcut_recording").catch(console.error);
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
      invoke("stop_shortcut_recording").catch(console.error);
    };
  }, [isRecording, finishRecording, cancelRecording]);

  // Real-time held-keys display (modifiers pass through the hook, so DOM events work)
  useEffect(() => {
    if (!isRecording) {
      setHeldKeys([]);
      return;
    }

    const pressed = new Set<string>();

    const toLabel = (e: KeyboardEvent): string => {
      if (MODIFIER_EVENT_KEYS.has(e.key)) return e.key;
      return normalizeKey(e.key, e.code);
    };

    const onKeyDown = (e: KeyboardEvent) => {
      pressed.add(toLabel(e));
      setHeldKeys([...pressed]);
    };
    const onKeyUp = (e: KeyboardEvent) => {
      pressed.delete(toLabel(e));
      setHeldKeys([...pressed]);
    };
    const onBlur = () => {
      pressed.clear();
      setHeldKeys([]);
    };

    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onBlur);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", onBlur);
    };
  }, [isRecording]);

  const startRecording = useCallback(() => setIsRecording(true), []);

  return { isRecording, heldKeys, startRecording, cancelRecording };
}
