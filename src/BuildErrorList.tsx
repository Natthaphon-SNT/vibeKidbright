/**
 * BuildErrorList — friendly build error panel.
 * Extracted from App.tsx so the rendering can be unit-tested in isolation.
 */

import type { ParsedBuildError } from "./errorHints";

interface BuildErrorListProps {
    errors: ParsedBuildError[];
    onJumpToError: (err: ParsedBuildError) => void;
    onAskAiFix: () => void;
}

export default function BuildErrorList({ errors, onJumpToError, onAskAiFix }: BuildErrorListProps) {
    if (errors.length === 0) return null;

    return (
        <div className="mx-2 mb-1 rounded-lg overflow-hidden animate-fadein" style={{ border: '1px solid var(--danger)', backgroundColor: 'rgba(185,28,28,0.05)' }}>
            <div className="px-3 py-2 flex items-center gap-2" style={{ borderBottom: '1px solid rgba(185,28,28,0.25)' }}>
                <span className="text-sm">⚠️</span>
                <span className="text-[12px] font-bold" style={{ color: 'var(--danger)' }}>
                    Build failed — {errors.length} problem{errors.length > 1 ? "s" : ""} found
                </span>
                <button
                    onClick={onAskAiFix}
                    className="ml-auto text-[11px] font-bold px-2.5 py-1 rounded-md transition-opacity hover:opacity-80 shrink-0"
                    style={{ backgroundColor: 'var(--accent)', color: '#ffffff' }}
                    title="ให้ Vibe Coder ช่วยแก้ / Let Vibe Coder fix it"
                >
                    🤖 Ask Vibe Coder to Fix
                </button>
            </div>
            <div className="max-h-44 overflow-y-auto">
                {errors.map((err, i) => (
                    <div key={i} className="px-3 py-2 flex items-start gap-2.5" style={{ borderBottom: i < errors.length - 1 ? '1px solid var(--border-color)' : undefined }}>
                        <span className="text-[11px] mt-0.5 shrink-0">❌</span>
                        <div className="min-w-0 flex-1">
                            <div className="flex items-center gap-2 flex-wrap">
                                <span className="text-[12px] font-bold" style={{ color: 'var(--danger)' }}>{err.title}</span>
                                {err.file && (
                                    <button
                                        onClick={() => onJumpToError(err)}
                                        className="text-[10px] font-mono px-1.5 py-0.5 rounded transition-colors hover:bg-red-500/20"
                                        style={{ color: 'var(--accent)', backgroundColor: 'var(--bg-hover)' }}
                                        title="เปิดไฟล์ตรงบรรทัดนี้ / Open at this line"
                                    >
                                        {err.file.split(/[\/\\]/).pop()}{err.line ? `:${err.line}` : ""}
                                    </button>
                                )}
                            </div>
                            <div className="text-[12px] mt-0.5 leading-relaxed">🇹🇭 {err.thaiHint}</div>
                            <div className="text-[10px] mt-0.5 opacity-70">{err.englishHint}</div>
                        </div>
                    </div>
                ))}
            </div>
        </div>
    );
}
