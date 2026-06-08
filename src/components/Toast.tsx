import { CircleX, Info, TriangleAlert, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

export interface ToastItem {
  id: number;
  message: string;
  type?: "error" | "warning" | "info";
}

let nextId = 0;
const listeners = new Set<(item: ToastItem) => void>();

export function showToast(message: string, type: ToastItem["type"] = "error") {
  const item: ToastItem = { id: nextId++, message, type };
  listeners.forEach((fn) => fn(item));
}
const AUTO_DISMISS_MS = 5000;
const EXIT_DURATION_MS = 200;

const iconComponents: Record<NonNullable<ToastItem["type"]>, React.ReactNode> = {
  warning: <TriangleAlert size={14} />,
  info: <Info size={14} />,
  error: <CircleX size={14} />,
};

const iconColor: Record<NonNullable<ToastItem["type"]>, string> = {
  error: "text-red-400",
  warning: "text-amber-400",
  info: "text-ink-faint",
};

function ToastEntry({ item, onDismiss }: { item: ToastItem; onDismiss: (id: number) => void }) {
  const [exiting, setExiting] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const dismiss = useCallback(() => {
    setExiting(true);
    timerRef.current = setTimeout(() => onDismiss(item.id), EXIT_DURATION_MS);
  }, [item.id, onDismiss]);

  useEffect(() => {
    timerRef.current = setTimeout(dismiss, AUTO_DISMISS_MS);
    return () => clearTimeout(timerRef.current);
  }, [dismiss]);

  const type = item.type ?? "error";

  return (
    <div className={`toast-entry ${exiting ? "toast-exit" : "toast-enter"}`} role="alert">
      <div className="flex items-start gap-2.5 min-w-0">
        <span className={`mt-px shrink-0 ${iconColor[type]}`}>{iconComponents[type]}</span>
        <span className="text-[12.5px] leading-relaxed text-ink-soft break-words min-w-0">
          {item.message}
        </span>
      </div>
      <button
        onClick={dismiss}
        className="shrink-0 mt-px ml-2 p-0.5 rounded text-ink-ghost hover:text-ink-faint transition-colors cursor-pointer"
      >
        <X size={12} />
      </button>
    </div>
  );
}

export function ToastContainer() {
  const [items, setItems] = useState<ToastItem[]>([]);

  useEffect(() => {
    const handler = (item: ToastItem) => setItems((prev) => [...prev.slice(-4), item]);
    listeners.add(handler);
    return () => {
      listeners.delete(handler);
    };
  }, []);

  const handleDismiss = useCallback((id: number) => {
    setItems((prev) => prev.filter((t) => t.id !== id));
  }, []);

  if (items.length === 0) return null;

  return (
    <div className="fixed top-12 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
      {items.map((item) => (
        <div key={item.id} className="pointer-events-auto">
          <ToastEntry item={item} onDismiss={handleDismiss} />
        </div>
      ))}
    </div>
  );
}
