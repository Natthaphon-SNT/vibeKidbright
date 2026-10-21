interface Props { allowed: boolean; onChange: (allowed: boolean) => void }
export default function PrivateContextConsent({ allowed, onChange }: Props) {
    return <label className="flex gap-2 text-xs mb-4" style={{ color: "var(--text-secondary)" }}>
        <input type="checkbox" checked={allowed} onChange={event => onChange(event.target.checked)} />
        <span><strong>Share private context with remote AI / อนุญาตส่งโค้ดและเอกสารส่วนตัวให้บริการ AI</strong>
            <span className="block mt-1">Enable only when you agree to send project/tool excerpts to the selected provider and configured fallback providers. Off: remote AI uses bundled public references; private file tools are blocked. Loopback local AI can use private context. Text you type or paste in chat is sent when you press Send.</span>
        </span>
    </label>;
}
