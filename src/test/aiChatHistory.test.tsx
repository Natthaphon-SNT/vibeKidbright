import { render, screen, fireEvent, cleanup, waitFor, act } from '@testing-library/react';
import { beforeEach, afterEach, it, expect, vi } from 'vitest';
import AiChat from '../AiChat';

const { invokeMock, handlers } = vi.hoisted(() => ({
    invokeMock: vi.fn(), handlers: new Map<string, Set<(event: { payload: string }) => void>>()
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invokeMock(...args) }));
vi.mock('@tauri-apps/api/event', () => ({ listen: (name: string, callback: (event: { payload: string }) => void) => {
    const callbacks = handlers.get(name) ?? new Set();
    callbacks.add(callback); handlers.set(name, callbacks);
    return Promise.resolve(() => { callbacks.delete(callback); });
} }));

const originalRequest = 'สร้าง mai_test_015 ทดสอบพิกัดทัช MP-015';
const offeredName = 'Folder exists. Use mai_test_015_v2?';
const props = { projectDir: '.', selectedBoard: 'miuaiplus' as const, onInjectCode: () => {} };
const sendCalls = () => invokeMock.mock.calls.filter(([command]) => command === 'send_ai_message');
async function send(text: string) {
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith('get_provider'));
    const input = screen.getByPlaceholderText('Ask about your code...');
    fireEvent.change(input, { target: { value: text } });
    fireEvent.keyDown(input, { key: 'Enter', shiftKey: false });
    await waitFor(() => expect(sendCalls().length).toBeGreaterThan(0));
}
async function emit(name: string, payload: unknown) {
    await act(async () => {
        for (const callback of handlers.get(name) ?? []) callback({ payload: typeof payload === 'string' ? payload : JSON.stringify(payload) });
    });
}
beforeEach(() => {
    localStorage.clear(); handlers.clear(); vi.clearAllMocks();
    invokeMock.mockImplementation(async (command: string) => {
        if (['get_knowledge_base_files', 'get_pending_project_reviews'].includes(command)) return [];
        if (command === 'get_private_context_consent') return false;
        if (command === 'get_provider') return 'local';
        if (command === 'get_base_url') return 'http://127.0.0.1:11434/v1';
        if (command === 'get_api_key') return 'test-only-key';
        return '';
    });
});
afterEach(cleanup);

it('shows provider token usage separately from request bytes and ignores stale reply events', async () => {
    render(<AiChat {...props} />);
    await send('read ADC0');
    const id=sendCalls()[0][1].messageId;
    await emit('ai-context-report',{message_id:id,request_bytes:24576,history:{compacted:true}});
    expect(screen.getByText('Context 24.0 KiB · older details compacted')).toBeInTheDocument();
    expect(screen.getByText('Tokens: awaiting provider usage')).toBeInTheDocument();
    await emit('ai-token-usage',{message_id:id,usage:{input_tokens:120,output_tokens:30,total_tokens:150}});
    await emit('ai-token-usage',{message_id:id,usage:{input_tokens:140,output_tokens:10,total_tokens:150}});
    expect(screen.getByText('Tokens this reply: 300 · input 260 · output 40')).toBeInTheDocument();
    await emit('ai-token-usage',{message_id:'stale',usage:{input_tokens:99999,output_tokens:1,total_tokens:100000}});
    expect(screen.getByText('Tokens this reply: 300 · input 260 · output 40')).toBeInTheDocument();
    await emit('ai-chat-done',{text:'ADC ready'});
    await send('continue');
    await waitFor(()=>expect(sendCalls()).toHaveLength(2));
    expect(screen.queryByText(/Tokens this reply: 300/)).not.toBeInTheDocument();
});

it('reports Google/local/custom usage, deduplicates request ids and labels missing counts', async () => {
    render(<AiChat {...props} />);
    await send('read ADC0');
    const id=sendCalls()[0][1].messageId;
    const google={message_id:id,request_id:'google-1',provider:'Google',model:'gemini-test',status:'reported',usage:{input_tokens:100,output_tokens:10,total_tokens:130,reasoning_tokens:20}};
    await emit('ai-token-usage',google);
    await emit('ai-token-usage',google);
    await emit('ai-token-usage',{message_id:id,request_id:'local-1',provider:'Local / OpenAI-compatible',model:'local-test',status:'unavailable',usage:null});
    expect(screen.getByText('Tokens this reply: 130 · input 100 · output 10 · thinking 20 · partial usage')).toBeInTheDocument();
    expect(screen.getByText('Google / Local / OpenAI-compatible')).toBeInTheDocument();
    await emit('ai-chat-done',{text:'done'});
    await send('continue');
    await waitFor(()=>expect(sendCalls()).toHaveLength(2));
    const nextId=sendCalls()[1][1].messageId;
    await emit('ai-token-usage',{message_id:nextId,request_id:'custom-2',provider:'Custom / OpenAI-compatible',status:'partial',usage:{input_tokens:7,output_tokens:null,total_tokens:null}});
    expect(screen.getByText('Tokens this reply: ? · input 7 · output ? · partial usage')).toBeInTheDocument();
    await emit('ai-chat-done',{text:'done'});
});

it('ends the awaiting state when a provider sends no usage for a completed reply', async () => {
    render(<AiChat {...props} />);
    await send('hello');
    await emit('ai-chat-done',{text:'Hello'});
    expect(screen.getByText('Tokens: not reported by provider')).toBeInTheDocument();
});

it('distinguishes a real UI board change from a short followup with the same selection', async () => {
    const view=render(<AiChat {...props} />);
    await send('use MAI Plus with IKB-1Z');
    expect(sendCalls()[0][1].selectedBoardChanged).toBe(true);
    await emit('ai-chat-done', {text:'Ready',history:[{role:'user',content:'use MAI Plus with IKB-1Z'}]});
    await send('continue');
    await waitFor(() => expect(sendCalls()).toHaveLength(2));
    expect(sendCalls()[1][1].selectedBoardChanged).toBe(false);
    await emit('ai-chat-done', {text:'Ready'});
    view.rerender(<AiChat {...props} selectedBoard="kidbright32" />);
    await send('create the program');
    await waitFor(() => expect(sendCalls()).toHaveLength(3));
    expect(sendCalls()[2][1].selectedBoard).toBe('kidbright32');
    expect(sendCalls()[2][1].selectedBoardChanged).toBe(true);
});

it('restores a saved request and offered name before sending the short v2 reply', async () => {
    localStorage.setItem('vibe_chat_sessions', JSON.stringify([{ id: 'saved', title: 'Touch', updatedAt: 1,
        messages: [{ id: 'u1', role: 'user', content: originalRequest }, { id: 'a1', role: 'assistant', content: offeredName,
            toolCalls: [{ name: 'read_file', result: 'PRIVATE_TOOL_RESULT' }] }] }]));
    render(<AiChat {...props} />);
    await screen.findByText(offeredName);
    await send('v2');
    expect(sendCalls()[0][1].messages).toEqual([
        { role: 'user', content: originalRequest }, { role: 'assistant', content: offeredName }, { role: 'user', content: 'v2' }
    ]);
    expect(JSON.stringify(sendCalls()[0][1].messages)).not.toContain('PRIVATE_TOOL_RESULT');
});

it('keeps complete live tool history without duplicating visible chat messages', async () => {
    render(<AiChat {...props} />);
    await send(originalRequest);
    const history = [
        { role: 'user', content: originalRequest },
        { role: 'assistant', content: { __tool_calls__: [{ id: 'call1', function: { name: 'create_project_workspace', arguments: '{}' } }] } },
        { role: 'tool', content: { __tool_response__: true, tool_call_id: 'call1', content: 'exists' } },
        { role: 'assistant', content: offeredName }
    ];
    await emit('ai-chat-done', { text: offeredName, history });
    await screen.findByText(offeredName);
    await send('v2');
    await waitFor(() => expect(sendCalls()).toHaveLength(2));
    expect(sendCalls()[1][1].messages).toEqual([...history, { role: 'user', content: 'v2' }]);
});

it('restores dialogue after a panel remount even when no streaming deltas arrived', async () => {
    const first = render(<AiChat {...props} />);
    await send(originalRequest);
    await emit('ai-chat-done', { text: offeredName, history: [{ role: 'user', content: originalRequest }, { role: 'assistant', content: offeredName }] });
    await screen.findByText(offeredName);
    first.unmount();
    render(<AiChat {...props} />);
    await screen.findByText(offeredName);
    await send('สร้างใหม่ชื่อนี้');
    await waitFor(() => expect(sendCalls()).toHaveLength(2));
    expect(sendCalls()[1][1].messages).toEqual([
        { role: 'user', content: originalRequest }, { role: 'assistant', content: offeredName }, { role: 'user', content: 'สร้างใหม่ชื่อนี้' }
    ]);
});

it('retains the failed user request after an asynchronous backend error', async () => {
    render(<AiChat {...props} />);
    await send(originalRequest);
    await emit('ai-chat-error', 'request failed');
    await send('ลองใหม่');
    await waitFor(() => expect(sendCalls()).toHaveLength(2));
    expect(sendCalls()[1][1].messages.filter((m: { role: string }) => m.role === 'user')).toEqual([
        { role: 'user', content: originalRequest }, { role: 'user', content: 'ลองใหม่' }
    ]);
});

it('discards stale tool results after workspace Undo while keeping the conversation', async () => {
    render(<AiChat {...props} />);
    await send(originalRequest);
    await emit('ai-chat-done', { text: offeredName, history: [
        { role: 'user', content: originalRequest }, { role: 'tool', content: 'STALE_WORKSPACE_RESULT' }, { role: 'assistant', content: offeredName }
    ] });
    await emit('ai-project-review-resolved', { action: 'undo' });
    await send('ใช้ชื่อ mai_test_015_v2');
    await waitFor(() => expect(sendCalls()).toHaveLength(2));
    const history = sendCalls()[1][1].messages;
    expect(history[0]).toEqual({ role: 'user', content: originalRequest });
    expect(JSON.stringify(history)).not.toContain('STALE_WORKSPACE_RESULT');
});
