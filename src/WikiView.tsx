// WikiView.tsx
// AI-Powered Wiki for vibeKidbright
// ─────────────────────────────────────────────────────────────────────────────
// หน้าที่ของ AI:
//  • สรุปบทความ (Summarize)
//  • จัดหมวดหมู่ / แนะนำ Tags
//  • สร้างลิงก์เชื่อมโยง Backlinks ระหว่างบทความ
//  • ตรวจหาความขัดแย้งของข้อมูล (Conflict Detection)
//  • สร้างบทความใหม่จาก prompt (AI Draft)
// ─────────────────────────────────────────────────────────────────────────────

import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ── Types ─────────────────────────────────────────────────────────────────────

interface WikiArticle {
  id: string;
  title: string;
  content: string; // Markdown
  tags: string[];
  backlinks: string[]; // IDs of articles linked to this one
  createdAt: number;
  updatedAt: number;
  aiSummary?: string;
}

interface AiSettings {
  provider: "openai" | "local" | "openrouter" | "google" | "zen";
  apiKey: string;
  baseUrl: string;
  model: string;
  openrouterApiKey: string;
  openrouterModel: string;
  googleApiKey: string;
  googleModel: string;
  zenApiKey: string;
  zenModel: string;
}

// ── Storage ───────────────────────────────────────────────────────────────────

const STORAGE_KEY = "vibe_wiki_articles";

function loadArticles(): WikiArticle[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as WikiArticle[];
  } catch {}
  return [];
}

function saveArticles(articles: WikiArticle[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(articles));
}

// ── Simple Markdown → HTML renderer ─────────────────────────────────────────

function renderMarkdown(md: string): string {
  if (!md) return "";
  let html = md
    // Code blocks
    .replace(/```(\w*)\n?([\s\S]*?)```/g, (_m, _lang, code) =>
      `<pre style="background:rgba(5,10,20,0.8);border:1px solid rgba(59,130,246,0.15);border-radius:8px;padding:12px;overflow-x:auto;font-size:12px;line-height:1.5;"><code>${code.replace(/</g, "&lt;")}</code></pre>`
    )
    // Inline code
    .replace(/`([^`]+)`/g, "<code style=\"background:rgba(59,130,246,0.12);border:1px solid rgba(59,130,246,0.2);border-radius:4px;padding:1px 5px;font-size:11px;font-family:monospace;\">$1</code>")
    // Headings
    .replace(/^### (.+)$/gm, "<h3 style=\"font-size:15px;font-weight:700;color:#93c5fd;margin:18px 0 8px;\">$1</h3>")
    .replace(/^## (.+)$/gm, "<h2 style=\"font-size:18px;font-weight:700;color:#bfdbfe;margin:22px 0 10px;border-bottom:1px solid rgba(59,130,246,0.2);padding-bottom:6px;\">$1</h2>")
    .replace(/^# (.+)$/gm, "<h1 style=\"font-size:22px;font-weight:800;color:#e0f2fe;margin:0 0 16px;border-bottom:2px solid rgba(59,130,246,0.3);padding-bottom:8px;\">$1</h1>")
    // Bold / Italic
    .replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>")
    .replace(/\*\*(.+?)\*\*/g, "<strong style=\"color:#f0f6ff;\">$1</strong>")
    .replace(/\*(.+?)\*/g, "<em style=\"color:#cbd5e1;\">$1</em>")
    // Wiki-style links [[Article Name]]
    .replace(/\[\[([^\]]+)\]\]/g, "<span style=\"color:#60a5fa;text-decoration:underline;cursor:pointer;font-weight:600;\">[[​$1]]</span>")
    // External links [text](url)
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, "<a href=\"$2\" target=\"_blank\" rel=\"noopener\" style=\"color:#38bdf8;text-decoration:underline;\">$1</a>")
    // Horizontal rule
    .replace(/^---$/gm, "<hr style=\"border:none;border-top:1px solid rgba(59,130,246,0.2);margin:20px 0;\"/>")
    // Blockquote
    .replace(/^> (.+)$/gm, "<blockquote style=\"border-left:3px solid rgba(59,130,246,0.5);padding-left:12px;margin:8px 0;color:#94a3b8;font-style:italic;\">$1</blockquote>")
    // Unordered list
    .replace(/^[\*\-] (.+)$/gm, "<li style=\"margin:3px 0;\">$1</li>")
    .replace(/(<li[^>]*>.*<\/li>\n?)+/g, (m) => `<ul style="list-style:disc;padding-left:20px;margin:8px 0;">${m}</ul>`)
    // Ordered list
    .replace(/^\d+\. (.+)$/gm, "<li style=\"margin:3px 0;\">$1</li>")
    // Line breaks
    .replace(/\n\n+/g, "<br/><br/>")
    .replace(/\n/g, "<br/>");

  return html;
}

// ── AI Caller ─────────────────────────────────────────────────────────────────

async function callAI(settings: AiSettings, systemPrompt: string, userPrompt: string): Promise<string> {
  let url = "";
  let headers: Record<string, string> = { "Content-Type": "application/json" };
  let model = "";
  let body: Record<string, unknown>;

  if (settings.provider === "google") {
    // Google Gemini API
    url = `https://generativelanguage.googleapis.com/v1beta/models/${settings.googleModel}:generateContent?key=${settings.googleApiKey}`;
    body = {
      contents: [{ parts: [{ text: `${systemPrompt}\n\n${userPrompt}` }] }],
      generationConfig: { maxOutputTokens: 2048 }
    };
    const resp = await fetch(url, { method: "POST", headers, body: JSON.stringify(body) });
    if (!resp.ok) throw new Error(`Google API error: ${resp.status} ${await resp.text()}`);
    const data = await resp.json();
    return data.candidates?.[0]?.content?.parts?.[0]?.text ?? "";
  }

  if (settings.provider === "openrouter") {
    url = "https://openrouter.ai/api/v1/chat/completions";
    model = settings.openrouterModel;
    headers["Authorization"] = `Bearer ${settings.openrouterApiKey}`;
    headers["HTTP-Referer"] = "https://vibekidbright.app";
  } else if (settings.provider === "zen") {
    url = "https://opencode.ai/zen/v1/chat/completions";
    model = settings.zenModel;
    headers["Authorization"] = `Bearer ${settings.zenApiKey}`;
  } else if (settings.provider === "local") {
    url = `${settings.baseUrl}/chat/completions`;
    model = settings.model;
  } else {
    url = `${settings.baseUrl}/chat/completions`;
    model = settings.model;
    headers["Authorization"] = `Bearer ${settings.apiKey}`;
  }

  body = {
    model,
    messages: [
      { role: "system", content: systemPrompt },
      { role: "user", content: userPrompt }
    ],
    max_tokens: 2048,
    temperature: 0.3
  };

  const resp = await fetch(url, { method: "POST", headers, body: JSON.stringify(body) });
  if (!resp.ok) throw new Error(`API error: ${resp.status} ${await resp.text()}`);
  const data = await resp.json();
  return data.choices?.[0]?.message?.content ?? "";
}

// ── WikiView Component ────────────────────────────────────────────────────────

export default function WikiView() {
  const [articles, setArticles] = useState<WikiArticle[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [editMode, setEditMode] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [newArticleTitle, setNewArticleTitle] = useState("");
  const [showNewModal, setShowNewModal] = useState(false);
  const [draftPrompt, setDraftPrompt] = useState("");

  // AI state
  const [aiLoading, setAiLoading] = useState<string | null>(null); // action name being processed
  const [aiResult, setAiResult] = useState<string>("");
  const [aiError, setAiError] = useState<string>("");
  const [aiSettings, setAiSettings] = useState<AiSettings>({
    provider: "openai",
    apiKey: "",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o",
    openrouterApiKey: "",
    openrouterModel: "google/gemini-2.5-flash:free",
    googleApiKey: "",
    googleModel: "gemini-2.5-flash",
    zenApiKey: "",
    zenModel: "nemotron-3.5-lightning-free",
  });

  // Editor state
  const [editorContent, setEditorContent] = useState("");
  const [editorTitle, setEditorTitle] = useState("");
  const [editorTags, setEditorTags] = useState("");
  const [showPreview, setShowPreview] = useState(false);
  const [hasUnsaved, setHasUnsaved] = useState(false);

  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Load from storage + AI settings on mount
  useEffect(() => {
    setArticles(loadArticles());

    // Load AI settings from Tauri backend
    Promise.all([
      invoke("get_provider").catch(() => "openai"),
      invoke("get_api_key").catch(() => ""),
      invoke("get_base_url").catch(() => "https://api.openai.com/v1"),
      invoke("get_model").catch(() => "gpt-4o"),
      invoke("get_openrouter_api_key").catch(() => ""),
      invoke("get_openrouter_model").catch(() => "google/gemini-2.5-flash:free"),
      invoke("get_google_api_key").catch(() => ""),
      invoke("get_google_model").catch(() => "gemini-2.5-flash"),
      invoke("get_zen_api_key").catch(() => ""),
      invoke("get_zen_model").catch(() => "nemotron-3.5-lightning-free"),
    ]).then(([provider, apiKey, baseUrl, model, orKey, orModel, gKey, gModel, zKey, zModel]) => {
      setAiSettings({
        provider: (provider as string) as AiSettings["provider"],
        apiKey: apiKey as string,
        baseUrl: baseUrl as string,
        model: model as string,
        openrouterApiKey: orKey as string,
        openrouterModel: orModel as string,
        googleApiKey: gKey as string,
        googleModel: gModel as string,
        zenApiKey: zKey as string,
        zenModel: zModel as string,
      });
    });
  }, []);

  // Persist articles when changed
  useEffect(() => {
    saveArticles(articles);
  }, [articles]);

  // Update editor fields when selected article changes
  useEffect(() => {
    if (selectedId) {
      const a = articles.find(x => x.id === selectedId);
      if (a) {
        setEditorContent(a.content);
        setEditorTitle(a.title);
        setEditorTags(a.tags.join(", "));
        setShowPreview(false);
        setEditMode(false);
        setHasUnsaved(false);
        setAiResult("");
        setAiError("");
      }
    }
  }, [selectedId]);

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current && editMode) {
      textareaRef.current.style.height = "auto";
      textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`;
    }
  }, [editorContent, editMode]);

  // Computed
  const selectedArticle = articles.find(a => a.id === selectedId) ?? null;

  const allTags = Array.from(new Set(articles.flatMap(a => a.tags))).sort();

  const filteredArticles = articles
    .filter(a => {
      const q = searchQuery.toLowerCase();
      const matchSearch = !q || a.title.toLowerCase().includes(q) || a.content.toLowerCase().includes(q) || a.tags.some(t => t.toLowerCase().includes(q));
      const matchTags = selectedTags.length === 0 || selectedTags.every(t => a.tags.includes(t));
      return matchSearch && matchTags;
    })
    .sort((a, b) => b.updatedAt - a.updatedAt);

  // ── CRUD ──────────────────────────────────────────────────────────────────

  const createArticle = () => {
    if (!newArticleTitle.trim()) return;
    const now = Date.now();
    const article: WikiArticle = {
      id: crypto.randomUUID(),
      title: newArticleTitle.trim(),
      content: `# ${newArticleTitle.trim()}\n\nเริ่มเขียนบทความที่นี่...`,
      tags: [],
      backlinks: [],
      createdAt: now,
      updatedAt: now,
    };
    setArticles(prev => [article, ...prev]);
    setSelectedId(article.id);
    setEditMode(true);
    setNewArticleTitle("");
    setShowNewModal(false);
  };

  const saveArticle = () => {
    if (!selectedId) return;
    const tags = editorTags.split(",").map(t => t.trim()).filter(Boolean);

    // Auto-detect backlinks: find [[ArticleName]] references and update backlinks
    const refPattern = /\[\[([^\]]+)\]\]/g;
    let match;
    const referencedTitles: string[] = [];
    while ((match = refPattern.exec(editorContent)) !== null) {
      referencedTitles.push(match[1].toLowerCase());
    }

    setArticles(prev => {
      const updated = prev.map(a => {
        if (a.id === selectedId) {
          return {
            ...a,
            title: editorTitle,
            content: editorContent,
            tags,
            updatedAt: Date.now(),
          };
        }
        // Update backlinks for referenced articles
        const isReferenced = referencedTitles.includes(a.title.toLowerCase());
        const alreadyLinked = a.backlinks.includes(selectedId);
        if (isReferenced && !alreadyLinked) {
          return { ...a, backlinks: [...a.backlinks, selectedId] };
        }
        if (!isReferenced && alreadyLinked) {
          return { ...a, backlinks: a.backlinks.filter(id => id !== selectedId) };
        }
        return a;
      });
      return updated;
    });
    setHasUnsaved(false);
  };

  const deleteArticle = (id: string) => {
    if (!window.confirm("ลบบทความนี้?")) return;
    setArticles(prev => prev.filter(a => a.id !== id).map(a => ({
      ...a,
      backlinks: a.backlinks.filter(bl => bl !== id),
    })));
    if (selectedId === id) setSelectedId(null);
  };

  // ── AI Functions ──────────────────────────────────────────────────────────

  const runAI = useCallback(async (action: string, prompt: string) => {
    setAiLoading(action);
    setAiResult("");
    setAiError("");
    try {
      const systemPrompt = `คุณเป็น AI ผู้ช่วยจัดการ Knowledge Base Wiki สำหรับ KidBright ESP32 IDE
ตอบเป็นภาษาไทยหรืออังกฤษขึ้นอยู่กับเนื้อหา ตอบกระชับ ชัดเจน และเป็นประโยชน์`;
      const result = await callAI(aiSettings, systemPrompt, prompt);
      setAiResult(result);

      // Auto-update summary in article
      if (action === "summarize" && selectedId) {
        setArticles(prev => prev.map(a =>
          a.id === selectedId ? { ...a, aiSummary: result } : a
        ));
      }
    } catch (err) {
      setAiError(String(err));
    } finally {
      setAiLoading(null);
    }
  }, [aiSettings, selectedId]);

  const aiSummarize = () => {
    if (!selectedArticle) return;
    runAI("summarize",
      `สรุปบทความต่อไปนี้ให้กระชับ อ่านง่าย ใน 3-5 ประโยค:\n\n# ${selectedArticle.title}\n\n${selectedArticle.content}`
    );
  };

  const aiSuggestTags = () => {
    if (!selectedArticle) return;
    runAI("tags",
      `วิเคราะห์บทความนี้แล้วแนะนำ Tags/หมวดหมู่ที่เหมาะสม 5-10 tags (ตอบเป็น JSON array เช่น ["ESP32", "GPIO", "hardware"]):\n\n# ${selectedArticle.title}\n\n${selectedArticle.content}`
    );
  };

  const aiFindBacklinks = () => {
    if (!selectedArticle || articles.length < 2) return;
    const otherTitles = articles.filter(a => a.id !== selectedArticle.id).map(a => `- "${a.title}"`).join("\n");
    runAI("backlinks",
      `บทความปัจจุบัน: "${selectedArticle.title}"\n\nบทความที่มีอยู่ใน Wiki:\n${otherTitles}\n\nวิเคราะห์ว่าบทความใดมีความเชื่อมโยงกับบทความปัจจุบัน และอธิบายว่าเชื่อมโยงกันอย่างไร ถ้าไม่มีความเชื่อมโยงให้บอกด้วย`
    );
  };

  const aiCheckConflicts = () => {
    if (!selectedArticle || articles.length < 2) return;
    const relatedArticles = articles
      .filter(a => a.id !== selectedArticle.id)
      .filter(a =>
        selectedArticle.tags.some(t => a.tags.includes(t)) ||
        a.title.toLowerCase().split(" ").some(w => w.length > 3 && selectedArticle.content.toLowerCase().includes(w))
      )
      .slice(0, 5);

    if (relatedArticles.length === 0) {
      setAiResult("ไม่พบบทความที่เกี่ยวข้องเพียงพอสำหรับการตรวจสอบความขัดแย้ง");
      return;
    }

    const context = relatedArticles.map(a => `=== ${a.title} ===\n${a.content.substring(0, 800)}`).join("\n\n");
    runAI("conflicts",
      `ตรวจสอบความขัดแย้งของข้อมูลระหว่างบทความปัจจุบันกับบทความอื่นๆ:\n\n**บทความปัจจุบัน: ${selectedArticle.title}**\n${selectedArticle.content}\n\n**บทความที่เกี่ยวข้อง:**\n${context}\n\nระบุข้อมูลที่ขัดแย้งกันโดยตรง หรือบอกว่าไม่พบความขัดแย้ง`
    );
  };

  const aiDraft = async () => {
    if (!draftPrompt.trim()) return;
    const title = draftPrompt.trim();
    const existingContext = articles.slice(0, 10).map(a => `- ${a.title}: ${a.aiSummary || a.content.substring(0, 100)}`).join("\n");

    setAiLoading("draft");
    setAiResult("");
    setAiError("");
    try {
      const systemPrompt = `คุณเป็นผู้เขียนบทความ Wiki สำหรับ KidBright ESP32 IDE เขียนเป็น Markdown ที่ชัดเจน มีตัวอย่างโค้ด และอ้างอิง [[บทความอื่น]] ด้วย Wiki-link syntax`;
      const userPrompt = `เขียนบทความ Wiki สำหรับหัวข้อ: "${title}"\n\nบทความที่มีอยู่แล้วใน Wiki:\n${existingContext || "(ยังไม่มีบทความ)"}\n\nเขียนบทความที่ครบถ้วนเป็น Markdown (ใช้ [[ชื่อบทความ]] สำหรับ Wiki-link)`;
      const result = await callAI(aiSettings, systemPrompt, userPrompt);

      // Auto-create article
      const now = Date.now();
      const article: WikiArticle = {
        id: crypto.randomUUID(),
        title,
        content: result,
        tags: [],
        backlinks: [],
        createdAt: now,
        updatedAt: now,
      };
      setArticles(prev => [article, ...prev]);
      setSelectedId(article.id);
      setEditMode(false);
      setDraftPrompt("");
      setAiResult(`✅ สร้างบทความ "${title}" เรียบร้อยแล้ว`);
    } catch (err) {
      setAiError(String(err));
    } finally {
      setAiLoading(null);
    }
  };

  const applyAiTags = () => {
    if (!aiResult || !selectedId) return;
    try {
      const match = aiResult.match(/\[([^\]]+)\]/);
      if (match) {
        const tags = JSON.parse(match[0]) as string[];
        setEditorTags(tags.join(", "));
        setHasUnsaved(true);
      }
    } catch {}
  };

  // ── Styles ────────────────────────────────────────────────────────────────
  const css = `
    @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap');
    .wiki-scrollbar::-webkit-scrollbar { width: 5px; }
    .wiki-scrollbar::-webkit-scrollbar-track { background: transparent; }
    .wiki-scrollbar::-webkit-scrollbar-thumb { background: rgba(59,130,246,0.25); border-radius: 100px; }
    .wiki-tag { display:inline-flex; align-items:center; gap:4px; padding:2px 8px; border-radius:100px; font-size:10px; font-weight:600; background:rgba(59,130,246,0.12); border:1px solid rgba(59,130,246,0.25); color:#93c5fd; cursor:pointer; transition:all 0.15s; }
    .wiki-tag:hover { background:rgba(59,130,246,0.25); }
    .wiki-tag.active { background:rgba(59,130,246,0.35); border-color:#3b82f6; color:#bfdbfe; }
    .wiki-article-item { padding:10px 12px; border-radius:10px; cursor:pointer; transition:all 0.15s; border:1px solid transparent; }
    .wiki-article-item:hover { background:rgba(59,130,246,0.06); border-color:rgba(59,130,246,0.1); }
    .wiki-article-item.active { background:rgba(59,130,246,0.1); border-color:rgba(59,130,246,0.25); }
    .wiki-ai-btn { width:100%; padding:8px 12px; border-radius:8px; border:1px solid rgba(59,130,246,0.2); background:rgba(59,130,246,0.06); color:#93c5fd; font-size:12px; font-weight:600; cursor:pointer; text-align:left; transition:all 0.15s; display:flex; align-items:center; gap:8px; }
    .wiki-ai-btn:hover:not(:disabled) { background:rgba(59,130,246,0.15); border-color:rgba(59,130,246,0.4); color:#bfdbfe; }
    .wiki-ai-btn:disabled { opacity:0.4; cursor:not-allowed; }
    .wiki-spinner { animation: spin 1s linear infinite; }
    @keyframes spin { 0%{transform:rotate(0deg)} 100%{transform:rotate(360deg)} }
    @keyframes wiki-fadein { from{opacity:0;transform:translateY(6px)} to{opacity:1;transform:translateY(0)} }
    .wiki-fadein { animation: wiki-fadein 0.25s ease forwards; }
    .wiki-editor { width:100%; min-height:300px; background:rgba(5,10,20,0.7); border:1px solid rgba(59,130,246,0.15); border-radius:10px; padding:14px; color:#e2e8f0; font-family:'JetBrains Mono',monospace; font-size:13px; line-height:1.7; resize:vertical; outline:none; transition:border-color 0.2s; }
    .wiki-editor:focus { border-color:rgba(59,130,246,0.5); box-shadow:0 0 0 3px rgba(59,130,246,0.08); }
  `;

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div style={{
      display: "flex",
      height: "100%",
      fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
      backgroundColor: "var(--bg-app)",
      color: "var(--text-primary)",
      overflow: "hidden",
    }}>
      <style>{css}</style>

      {/* ── LEFT: Article List Sidebar ──────────────────────────────────── */}
      <div style={{
        width: "260px",
        flexShrink: 0,
        display: "flex",
        flexDirection: "column",
        borderRight: "1px solid var(--border-color)",
        backgroundColor: "var(--bg-sidebar)",
      }}>
        {/* Header */}
        <div style={{ padding: "16px", borderBottom: "1px solid var(--border-color)" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "10px" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
              <span style={{ fontSize: "18px" }}>📚</span>
              <span style={{ fontSize: "14px", fontWeight: 700, background: "linear-gradient(135deg,#60a5fa,#a78bfa)", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
                AI Wiki
              </span>
            </div>
            <button
              onClick={() => setShowNewModal(true)}
              title="New Article"
              style={{
                width: "28px", height: "28px", borderRadius: "8px", border: "none",
                background: "rgba(59,130,246,0.15)", color: "#60a5fa",
                fontSize: "18px", cursor: "pointer", display: "flex", alignItems: "center", justifyContent: "center",
                lineHeight: 1, transition: "all 0.15s",
              }}
              onMouseEnter={e => { (e.currentTarget as HTMLElement).style.background = "rgba(59,130,246,0.3)"; }}
              onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "rgba(59,130,246,0.15)"; }}
            >+</button>
          </div>

          {/* Search */}
          <div style={{ position: "relative" }}>
            <svg style={{ position: "absolute", left: "8px", top: "50%", transform: "translateY(-50%)" }} width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#64748b" strokeWidth="2">
              <circle cx="11" cy="11" r="8"/><path d="M21 21l-4.35-4.35"/>
            </svg>
            <input
              type="text"
              placeholder="ค้นหาบทความ..."
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              style={{
                width: "100%", boxSizing: "border-box", paddingLeft: "28px", paddingRight: "10px",
                paddingTop: "7px", paddingBottom: "7px",
                background: "rgba(10,15,25,0.6)", border: "1px solid rgba(59,130,246,0.15)",
                borderRadius: "8px", color: "var(--text-primary)", fontSize: "12px", outline: "none",
              }}
            />
          </div>
        </div>

        {/* Tags Filter */}
        {allTags.length > 0 && (
          <div style={{ padding: "8px 12px", borderBottom: "1px solid var(--border-color)", display: "flex", flexWrap: "wrap", gap: "4px" }}>
            {allTags.map(tag => (
              <span
                key={tag}
                className={`wiki-tag${selectedTags.includes(tag) ? " active" : ""}`}
                onClick={() => setSelectedTags(prev =>
                  prev.includes(tag) ? prev.filter(t => t !== tag) : [...prev, tag]
                )}
              >#{tag}</span>
            ))}
          </div>
        )}

        {/* Article list */}
        <div className="wiki-scrollbar" style={{ flex: 1, overflowY: "auto", padding: "8px" }}>
          {filteredArticles.length === 0 ? (
            <div style={{ padding: "20px", textAlign: "center", color: "#475569", fontSize: "12px" }}>
              {articles.length === 0 ? (
                <>
                  <div style={{ fontSize: "32px", marginBottom: "8px" }}>📝</div>
                  <p>ยังไม่มีบทความ<br/>กด + เพื่อสร้างบทความแรก</p>
                </>
              ) : "ไม่พบบทความที่ตรงกัน"}
            </div>
          ) : filteredArticles.map(a => (
            <div
              key={a.id}
              className={`wiki-article-item${a.id === selectedId ? " active" : ""}`}
              onClick={() => { setSelectedId(a.id); setEditMode(false); }}
            >
              <div style={{ fontSize: "13px", fontWeight: 600, color: a.id === selectedId ? "#bfdbfe" : "var(--text-primary)", marginBottom: "4px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {a.title}
              </div>
              {a.aiSummary && (
                <div style={{ fontSize: "10px", color: "#64748b", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", marginBottom: "4px" }}>
                  🤖 {a.aiSummary.substring(0, 60)}...
                </div>
              )}
              <div style={{ display: "flex", flexWrap: "wrap", gap: "3px", marginBottom: "4px" }}>
                {a.tags.slice(0, 3).map(t => (
                  <span key={t} style={{ fontSize: "9px", padding: "1px 5px", background: "rgba(59,130,246,0.1)", borderRadius: "100px", color: "#64748b" }}>#{t}</span>
                ))}
                {a.tags.length > 3 && <span style={{ fontSize: "9px", color: "#475569" }}>+{a.tags.length - 3}</span>}
              </div>
              <div style={{ fontSize: "9px", color: "#334155" }}>
                {new Date(a.updatedAt).toLocaleDateString("th-TH")}
                {a.backlinks.length > 0 && ` · 🔗 ${a.backlinks.length}`}
              </div>
            </div>
          ))}
        </div>

        {/* Stats */}
        <div style={{ padding: "8px 12px", borderTop: "1px solid var(--border-color)", fontSize: "10px", color: "#334155" }}>
          {articles.length} บทความ · {allTags.length} tags
        </div>
      </div>

      {/* ── CENTER: Editor / Viewer ─────────────────────────────────────── */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0, overflow: "hidden" }}>
        {selectedArticle ? (
          <>
            {/* Toolbar */}
            <div style={{
              height: "44px", display: "flex", alignItems: "center", justifyContent: "space-between",
              padding: "0 16px", borderBottom: "1px solid var(--border-color)",
              backgroundColor: "var(--bg-panel)", flexShrink: 0,
            }}>
              <div style={{ display: "flex", alignItems: "center", gap: "8px", minWidth: 0 }}>
                {editMode ? (
                  <input
                    value={editorTitle}
                    onChange={e => { setEditorTitle(e.target.value); setHasUnsaved(true); }}
                    style={{
                      background: "rgba(10,15,25,0.6)", border: "1px solid rgba(59,130,246,0.3)",
                      borderRadius: "6px", padding: "4px 8px", color: "var(--text-primary)",
                      fontSize: "14px", fontWeight: 700, outline: "none", minWidth: "200px",
                    }}
                  />
                ) : (
                  <span style={{ fontSize: "14px", fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                    {selectedArticle.title}
                  </span>
                )}
                {hasUnsaved && <span style={{ width: "7px", height: "7px", borderRadius: "50%", backgroundColor: "#fbbf24", flexShrink: 0 }} title="มีการแก้ไขที่ยังไม่ได้บันทึก" />}
              </div>

              <div style={{ display: "flex", alignItems: "center", gap: "6px", flexShrink: 0 }}>
                {editMode && (
                  <>
                    <button
                      onClick={() => setShowPreview(v => !v)}
                      style={{
                        padding: "4px 10px", borderRadius: "6px", border: "1px solid rgba(59,130,246,0.2)",
                        background: showPreview ? "rgba(59,130,246,0.15)" : "transparent",
                        color: "#60a5fa", fontSize: "11px", fontWeight: 600, cursor: "pointer",
                      }}
                    >
                      {showPreview ? "✏️ Edit" : "👁 Preview"}
                    </button>
                    <button
                      onClick={saveArticle}
                      style={{
                        padding: "4px 12px", borderRadius: "6px",
                        background: "rgba(16,185,129,0.15)", color: "#10b981",
                        fontSize: "11px", fontWeight: 700, cursor: "pointer",
                        border: "1px solid rgba(16,185,129,0.3)",
                      }}
                    >
                      💾 Save
                    </button>
                  </>
                )}
                <button
                  onClick={() => {
                    if (editMode && hasUnsaved) {
                      if (!window.confirm("ยกเลิกการแก้ไข?")) return;
                      const a = articles.find(x => x.id === selectedId);
                      if (a) { setEditorContent(a.content); setEditorTitle(a.title); setEditorTags(a.tags.join(", ")); }
                      setHasUnsaved(false);
                    }
                    setEditMode(v => !v);
                  }}
                  style={{
                    padding: "4px 10px", borderRadius: "6px",
                    border: "1px solid rgba(148,163,184,0.15)",
                    background: editMode ? "rgba(148,163,184,0.08)" : "rgba(59,130,246,0.08)",
                    color: editMode ? "#94a3b8" : "#60a5fa",
                    fontSize: "11px", fontWeight: 600, cursor: "pointer",
                  }}
                >
                  {editMode ? "✕ Cancel" : "✏️ Edit"}
                </button>
                <button
                  onClick={() => deleteArticle(selectedArticle.id)}
                  style={{
                    padding: "4px 8px", borderRadius: "6px", border: "1px solid rgba(239,68,68,0.2)",
                    background: "rgba(239,68,68,0.06)", color: "#f87171",
                    fontSize: "11px", cursor: "pointer",
                  }}
                  title="ลบบทความ"
                >🗑️</button>
              </div>
            </div>

            {/* Tags Editor */}
            {editMode && (
              <div style={{ padding: "8px 16px", borderBottom: "1px solid var(--border-color)", backgroundColor: "rgba(5,10,20,0.4)", display: "flex", alignItems: "center", gap: "8px" }}>
                <span style={{ fontSize: "11px", color: "#475569", fontWeight: 600 }}>Tags:</span>
                <input
                  value={editorTags}
                  onChange={e => { setEditorTags(e.target.value); setHasUnsaved(true); }}
                  placeholder="esp32, gpio, hardware (คั่นด้วยจุลภาค)"
                  style={{
                    flex: 1, background: "rgba(10,15,25,0.5)", border: "1px solid rgba(59,130,246,0.15)",
                    borderRadius: "6px", padding: "4px 10px", color: "var(--text-primary)",
                    fontSize: "12px", outline: "none",
                  }}
                />
              </div>
            )}

            {/* Content Area */}
            <div className="wiki-scrollbar" style={{ flex: 1, overflowY: "auto", padding: "24px" }}>
              {editMode && !showPreview ? (
                <textarea
                  ref={textareaRef}
                  className="wiki-editor"
                  value={editorContent}
                  onChange={e => { setEditorContent(e.target.value); setHasUnsaved(true); }}
                  placeholder="เขียนบทความเป็น Markdown... ใช้ [[ชื่อบทความ]] สำหรับ Wiki-link"
                />
              ) : (
                <div
                  className="wiki-fadein"
                  style={{ maxWidth: "720px", lineHeight: 1.8, color: "#cbd5e1" }}
                  dangerouslySetInnerHTML={{ __html: renderMarkdown(selectedArticle.content) }}
                />
              )}
            </div>

            {/* Backlinks Footer */}
            {selectedArticle.backlinks.length > 0 && !editMode && (
              <div style={{ padding: "10px 24px", borderTop: "1px solid var(--border-color)", display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap", flexShrink: 0 }}>
                <span style={{ fontSize: "11px", color: "#475569", fontWeight: 600 }}>🔗 Referenced by:</span>
                {selectedArticle.backlinks.map(id => {
                  const a = articles.find(x => x.id === id);
                  return a ? (
                    <span
                      key={id}
                      onClick={() => setSelectedId(id)}
                      style={{ fontSize: "11px", color: "#60a5fa", cursor: "pointer", textDecoration: "underline" }}
                    >{a.title}</span>
                  ) : null;
                })}
              </div>
            )}
          </>
        ) : (
          /* Empty State */
          <div style={{ flex: 1, display: "flex", alignItems: "center", justifyContent: "center" }}>
            <div style={{ textAlign: "center", opacity: 0.4 }}>
              <div style={{ fontSize: "64px", marginBottom: "16px" }}>📚</div>
              <p style={{ fontSize: "16px", fontWeight: 600, marginBottom: "8px" }}>เลือกบทความจาก Sidebar</p>
              <p style={{ fontSize: "12px", color: "#475569" }}>หรือกด + เพื่อสร้างบทความใหม่</p>
            </div>
          </div>
        )}
      </div>

      {/* ── RIGHT: AI Panel ─────────────────────────────────────────────── */}
      <div style={{
        width: "280px", flexShrink: 0, display: "flex", flexDirection: "column",
        borderLeft: "1px solid var(--border-color)", backgroundColor: "var(--bg-sidebar)", overflow: "hidden",
      }}>
        {/* AI Panel Header */}
        <div style={{ padding: "12px 14px", borderBottom: "1px solid var(--border-color)", flexShrink: 0 }}>
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <span style={{ fontSize: "16px" }}>✦</span>
            <span style={{ fontSize: "13px", fontWeight: 700, background: "linear-gradient(135deg,#a78bfa,#60a5fa)", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent" }}>
              AI Assistant
            </span>
          </div>
          <p style={{ fontSize: "10px", color: "#334155", margin: "4px 0 0" }}>
            {aiSettings.provider === "google" ? `Gemini · ${aiSettings.googleModel}` :
             aiSettings.provider === "openrouter" ? `OpenRouter · ${aiSettings.openrouterModel}` :
             aiSettings.provider === "zen" ? `Zen · ${aiSettings.zenModel}` :
             aiSettings.provider === "local" ? "Local LLM" :
             `OpenAI · ${aiSettings.model}`}
          </p>
        </div>

        <div className="wiki-scrollbar" style={{ flex: 1, overflowY: "auto", padding: "12px" }}>
          {/* AI Actions for selected article */}
          {selectedArticle ? (
            <>
              <p style={{ fontSize: "10px", color: "#475569", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "8px" }}>
                ARTICLE ACTIONS
              </p>

              <div style={{ display: "flex", flexDirection: "column", gap: "6px", marginBottom: "16px" }}>
                <button
                  className="wiki-ai-btn"
                  onClick={aiSummarize}
                  disabled={!!aiLoading}
                >
                  {aiLoading === "summarize" ? <svg className="wiki-spinner" width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="rgba(148,163,184,0.3)" strokeWidth="3"/><path d="M12 2a10 10 0 0110 10" stroke="#3b82f6" strokeWidth="3" strokeLinecap="round"/></svg> : "📝"}
                  Summarize Article
                </button>

                <button
                  className="wiki-ai-btn"
                  onClick={aiSuggestTags}
                  disabled={!!aiLoading}
                >
                  {aiLoading === "tags" ? <svg className="wiki-spinner" width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="rgba(148,163,184,0.3)" strokeWidth="3"/><path d="M12 2a10 10 0 0110 10" stroke="#3b82f6" strokeWidth="3" strokeLinecap="round"/></svg> : "🏷️"}
                  Suggest Tags
                </button>

                <button
                  className="wiki-ai-btn"
                  onClick={aiFindBacklinks}
                  disabled={!!aiLoading || articles.length < 2}
                  title={articles.length < 2 ? "ต้องมีบทความอย่างน้อย 2 บทความ" : ""}
                >
                  {aiLoading === "backlinks" ? <svg className="wiki-spinner" width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="rgba(148,163,184,0.3)" strokeWidth="3"/><path d="M12 2a10 10 0 0110 10" stroke="#3b82f6" strokeWidth="3" strokeLinecap="round"/></svg> : "🔗"}
                  Find Backlinks
                </button>

                <button
                  className="wiki-ai-btn"
                  onClick={aiCheckConflicts}
                  disabled={!!aiLoading || articles.length < 2}
                  title={articles.length < 2 ? "ต้องมีบทความอย่างน้อย 2 บทความ" : ""}
                >
                  {aiLoading === "conflicts" ? <svg className="wiki-spinner" width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="rgba(148,163,184,0.3)" strokeWidth="3"/><path d="M12 2a10 10 0 0110 10" stroke="#3b82f6" strokeWidth="3" strokeLinecap="round"/></svg> : "⚠️"}
                  Check Conflicts
                </button>
              </div>

              {/* AI Result area */}
              {(aiResult || aiError || aiLoading) && (
                <div className="wiki-fadein" style={{
                  background: aiError ? "rgba(239,68,68,0.06)" : "rgba(59,130,246,0.05)",
                  border: `1px solid ${aiError ? "rgba(239,68,68,0.2)" : "rgba(59,130,246,0.15)"}`,
                  borderRadius: "10px", padding: "12px", marginBottom: "12px",
                }}>
                  {aiLoading && (
                    <div style={{ display: "flex", alignItems: "center", gap: "8px", color: "#60a5fa", fontSize: "12px" }}>
                      <svg className="wiki-spinner" width="12" height="12" viewBox="0 0 24 24" fill="none">
                        <circle cx="12" cy="12" r="10" stroke="rgba(148,163,184,0.2)" strokeWidth="3"/>
                        <path d="M12 2a10 10 0 0110 10" stroke="#3b82f6" strokeWidth="3" strokeLinecap="round"/>
                      </svg>
                      กำลังประมวลผล...
                    </div>
                  )}
                  {aiError && (
                    <p style={{ fontSize: "11px", color: "#f87171", margin: 0, wordBreak: "break-all" }}>{aiError}</p>
                  )}
                  {aiResult && !aiLoading && (
                    <>
                      <p style={{ fontSize: "11px", color: "#94a3b8", margin: "0 0 8px", lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                        {aiResult}
                      </p>
                      {/* Apply Buttons */}
                      {aiResult.includes("[") && aiResult.includes("]") && (
                        <button
                          onClick={applyAiTags}
                          style={{
                            padding: "4px 10px", borderRadius: "6px", border: "1px solid rgba(59,130,246,0.3)",
                            background: "rgba(59,130,246,0.1)", color: "#60a5fa",
                            fontSize: "10px", fontWeight: 700, cursor: "pointer",
                          }}
                        >
                          ← Apply Tags to Article
                        </button>
                      )}
                    </>
                  )}
                </div>
              )}

              {/* Show existing summary */}
              {selectedArticle.aiSummary && !aiResult && (
                <div style={{
                  background: "rgba(167,139,250,0.05)", border: "1px solid rgba(167,139,250,0.15)",
                  borderRadius: "10px", padding: "10px", marginBottom: "12px",
                }}>
                  <p style={{ fontSize: "10px", color: "#64748b", fontWeight: 600, margin: "0 0 4px" }}>📝 AI Summary</p>
                  <p style={{ fontSize: "11px", color: "#94a3b8", margin: 0, lineHeight: 1.6 }}>
                    {selectedArticle.aiSummary}
                  </p>
                </div>
              )}
            </>
          ) : (
            <p style={{ fontSize: "12px", color: "#334155", textAlign: "center", marginTop: "20px" }}>
              เลือกบทความเพื่อใช้งาน AI
            </p>
          )}

          {/* Divider */}
          <div style={{ borderTop: "1px solid var(--border-color)", margin: "8px 0 12px" }} />

          {/* AI Draft */}
          <p style={{ fontSize: "10px", color: "#475569", fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.5px", marginBottom: "8px" }}>
            ✨ AI DRAFT
          </p>
          <p style={{ fontSize: "11px", color: "#334155", marginBottom: "8px", lineHeight: 1.5 }}>
            ให้ AI เขียนบทความใหม่ให้อัตโนมัติ
          </p>
          <textarea
            value={draftPrompt}
            onChange={e => setDraftPrompt(e.target.value)}
            placeholder="ระบุหัวข้อบทความ เช่น 'การใช้งาน OLED SSD1306 กับ KidBright'"
            style={{
              width: "100%", boxSizing: "border-box", height: "70px",
              background: "rgba(10,15,20,0.6)", border: "1px solid rgba(59,130,246,0.15)",
              borderRadius: "8px", padding: "8px", color: "var(--text-primary)",
              fontSize: "11px", outline: "none", resize: "vertical", lineHeight: 1.5,
              fontFamily: "inherit", marginBottom: "6px",
            }}
          />
          <button
            className="wiki-ai-btn"
            onClick={aiDraft}
            disabled={!!aiLoading || !draftPrompt.trim()}
            style={{ justifyContent: "center" }}
          >
            {aiLoading === "draft" ? <svg className="wiki-spinner" width="12" height="12" viewBox="0 0 24 24" fill="none"><circle cx="12" cy="12" r="10" stroke="rgba(148,163,184,0.3)" strokeWidth="3"/><path d="M12 2a10 10 0 0110 10" stroke="#3b82f6" strokeWidth="3" strokeLinecap="round"/></svg> : "✦"}
            Generate Article with AI
          </button>
        </div>
      </div>

      {/* ── New Article Modal ───────────────────────────────────────────── */}
      {showNewModal && (
        <div style={{
          position: "fixed", inset: 0, zIndex: 9999,
          backgroundColor: "rgba(0,0,0,0.6)", backdropFilter: "blur(8px)",
          display: "flex", alignItems: "center", justifyContent: "center",
        }}>
          <div className="wiki-fadein" style={{
            width: "400px", background: "rgba(12,18,30,0.98)",
            border: "1px solid rgba(59,130,246,0.25)", borderRadius: "16px",
            padding: "28px", boxShadow: "0 24px 64px rgba(0,0,0,0.6)",
          }}>
            <h3 style={{ fontSize: "16px", fontWeight: 700, margin: "0 0 16px", color: "#e0f2fe" }}>
              📝 สร้างบทความใหม่
            </h3>
            <input
              autoFocus
              type="text"
              value={newArticleTitle}
              onChange={e => setNewArticleTitle(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter") createArticle(); if (e.key === "Escape") setShowNewModal(false); }}
              placeholder="ชื่อบทความ เช่น การใช้งาน GPIO"
              style={{
                width: "100%", boxSizing: "border-box",
                background: "rgba(10,15,25,0.8)", border: "1px solid rgba(59,130,246,0.3)",
                borderRadius: "10px", padding: "12px 14px", color: "#e2e8f0",
                fontSize: "14px", outline: "none", marginBottom: "16px",
              }}
            />
            <div style={{ display: "flex", gap: "10px" }}>
              <button
                onClick={() => { setShowNewModal(false); setNewArticleTitle(""); }}
                style={{
                  flex: 1, padding: "10px", borderRadius: "10px",
                  border: "1px solid rgba(148,163,184,0.15)",
                  background: "transparent", color: "#64748b",
                  fontSize: "13px", cursor: "pointer",
                }}
              >ยกเลิก</button>
              <button
                onClick={createArticle}
                disabled={!newArticleTitle.trim()}
                style={{
                  flex: 1, padding: "10px", borderRadius: "10px", border: "none",
                  background: newArticleTitle.trim() ? "linear-gradient(135deg,#1d4ed8,#3b82f6)" : "rgba(30,40,60,0.5)",
                  color: newArticleTitle.trim() ? "white" : "#475569",
                  fontSize: "13px", fontWeight: 700,
                  cursor: newArticleTitle.trim() ? "pointer" : "not-allowed",
                }}
              >สร้างบทความ</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
