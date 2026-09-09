/**
 * BuildErrorList — floating, collapsible build error panel.
 * Always stays on top of the editor so users can click through errors
 * without losing access to the error list.
 */

import { useState } from "react";
import type { ParsedBuildError } from "./errorHints";

interface BuildErrorListProps {
    errors: ParsedBuildError[];
    onJumpToError: (err: ParsedBuildError) => void;
    onAskAiFix: () => void;
}

export default function BuildErrorList({ errors, onJumpToError, onAskAiFix }: BuildErrorListProps) {
    const [expanded, setExpanded] = useState(true);

    if (errors.length === 0) return null;

    return (
        <div
            style={{
                position: 'absolute',
                top: 0,
                left: 0,
                right: 0,
                zIndex: 50,
                pointerEvents: 'none',
            }}
        >
            <div
                className="animate-fadein"
                style={{
                    margin: '8px 8px 0',
                    borderRadius: '10px',
                    border: '1px solid var(--danger)',
                    backgroundColor: 'var(--bg-sidebar)',
                    boxShadow: '0 8px 32px rgba(0,0,0,0.35), 0 2px 8px rgba(185,28,28,0.2)',
                    overflow: 'hidden',
                    pointerEvents: 'auto',
                    backdropFilter: 'blur(12px)',
                }}
            >
                {/* Header — always visible, acts as toggle */}
                <button
                    onClick={() => setExpanded(prev => !prev)}
                    style={{
                        width: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '8px',
                        padding: '8px 12px',
                        border: 'none',
                        borderBottom: expanded ? '1px solid rgba(185,28,28,0.25)' : 'none',
                        background: 'rgba(185,28,28,0.08)',
                        cursor: 'pointer',
                        color: 'inherit',
                        textAlign: 'left',
                    }}
                >
                    {/* Chevron */}
                    <svg
                        width="12" height="12" viewBox="0 0 24 24" fill="none"
                        stroke="var(--danger)" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round"
                        style={{
                            transition: 'transform 0.2s ease',
                            transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
                            flexShrink: 0,
                        }}
                    >
                        <path d="M9 18l6-6-6-6" />
                    </svg>

                    <span style={{ fontSize: '12px', fontWeight: 700, color: 'var(--danger)' }}>
                        ⚠️ Build failed — {errors.length} problem{errors.length > 1 ? "s" : ""} found
                    </span>

                    {/* Ask AI button */}
                    <span
                        onClick={(e) => { e.stopPropagation(); onAskAiFix(); }}
                        className="transition-opacity hover:opacity-80"
                        style={{
                            marginLeft: 'auto',
                            fontSize: '11px',
                            fontWeight: 700,
                            padding: '3px 10px',
                            borderRadius: '6px',
                            backgroundColor: 'var(--accent)',
                            color: '#ffffff',
                            cursor: 'pointer',
                            flexShrink: 0,
                        }}
                        title="ให้ Vibe Coder ช่วยแก้ / Let Vibe Coder fix it"
                    >
                        🤖 Ask Vibe Coder to Fix
                    </span>
                </button>

                {/* Error list — collapsible */}
                {expanded && (
                    <div style={{ maxHeight: '220px', overflowY: 'auto' }}>
                        {errors.map((err, i) => (
                            <div
                                key={i}
                                className="transition-colors"
                                onClick={() => onJumpToError(err)}
                                style={{
                                    display: 'flex',
                                    alignItems: 'flex-start',
                                    gap: '10px',
                                    padding: '8px 12px',
                                    borderBottom: i < errors.length - 1 ? '1px solid var(--border-color)' : undefined,
                                    cursor: err.file ? 'pointer' : 'default',
                                }}
                                onMouseEnter={e => { e.currentTarget.style.backgroundColor = 'rgba(185,28,28,0.08)'; }}
                                onMouseLeave={e => { e.currentTarget.style.backgroundColor = ''; }}
                            >
                                <span style={{ fontSize: '11px', marginTop: '2px', flexShrink: 0 }}>❌</span>
                                <div style={{ minWidth: 0, flex: 1 }}>
                                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px', flexWrap: 'wrap' }}>
                                        <span style={{ fontSize: '12px', fontWeight: 700, color: 'var(--danger)' }}>
                                            {err.title}
                                        </span>
                                        {err.file && (
                                            <span
                                                style={{
                                                    fontSize: '10px',
                                                    fontFamily: 'monospace',
                                                    padding: '2px 6px',
                                                    borderRadius: '4px',
                                                    color: 'var(--accent)',
                                                    backgroundColor: 'var(--bg-hover)',
                                                }}
                                                title="Click to jump to this line"
                                            >
                                                📄 {err.file.split(/[/\\]/).pop()}{err.line ? `:${err.line}` : ""}
                                            </span>
                                        )}
                                    </div>
                                    <div style={{ fontSize: '12px', marginTop: '3px', lineHeight: 1.5 }}>🇹🇭 {err.thaiHint}</div>
                                    <div style={{ fontSize: '10px', marginTop: '2px', opacity: 0.7 }}>{err.englishHint}</div>
                                </div>
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
