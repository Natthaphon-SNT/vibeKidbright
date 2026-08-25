/**
 * Toast — lightweight global toast notifications.
 *
 * Usage:
 *   import { toast, ToastHost } from "./Toast";
 *   <ToastHost />            // mount once near the app root
 *   toast("Failed to save", "error");
 */

import { useEffect, useState } from "react";

export type ToastType = "error" | "info" | "success";

interface ToastItem {
    id: number;
    message: string;
    type: ToastType;
}

type PushFn = (t: ToastItem) => void;

let pushFn: PushFn | null = null;
let nextId = 1;

export function toast(message: string, type: ToastType = "error") {
    const item: ToastItem = { id: nextId++, message, type };
    if (pushFn) {
        pushFn(item);
    } else {
        // Host not mounted yet — fall back to console so nothing is lost.
        console[type === "error" ? "error" : "log"](`[toast] ${message}`);
    }
}

const TYPE_STYLES: Record<ToastType, { icon: string; border: string }> = {
    error: { icon: "❌", border: "#b91c1c" },
    info: { icon: "ℹ️", border: "#1d4ed8" },
    success: { icon: "✅", border: "#15803d" },
};

const AUTO_DISMISS_MS = 6000;

export function ToastHost() {
    const [items, setItems] = useState<ToastItem[]>([]);

    useEffect(() => {
        pushFn = (t: ToastItem) => {
            setItems(prev => [...prev.slice(-4), t]);
            window.setTimeout(() => {
                setItems(prev => prev.filter(i => i.id !== t.id));
            }, AUTO_DISMISS_MS);
        };
        return () => { pushFn = null; };
    }, []);

    if (items.length === 0) return null;

    return (
        <div
            data-testid="toast-host"
            className="fixed bottom-4 right-4 z-[9999] flex flex-col gap-2 max-w-sm"
        >
            {items.map(item => {
                const style = TYPE_STYLES[item.type];
                return (
                    <div
                        key={item.id}
                        role="status"
                        className="flex items-start gap-2 px-3 py-2 rounded-lg shadow-lg backdrop-blur-sm text-[12px] leading-relaxed animate-fadein"
                        style={{
                            backgroundColor: 'var(--bg-modal, #fff)',
                            border: `1px solid ${style.border}`,
                            color: 'var(--text-color, inherit)',
                        }}
                    >
                        <span className="shrink-0">{style.icon}</span>
                        <span className="min-w-0 break-words">{item.message}</span>
                        <button
                            onClick={() => setItems(prev => prev.filter(i => i.id !== item.id))}
                            className="ml-auto shrink-0 opacity-50 hover:opacity-100 transition-opacity"
                            title="Dismiss"
                        >
                            ✕
                        </button>
                    </div>
                );
            })}
        </div>
    );
}
