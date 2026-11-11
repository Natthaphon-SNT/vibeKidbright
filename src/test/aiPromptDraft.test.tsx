import {StrictMode} from 'react';
import {render,screen,fireEvent,cleanup,waitFor} from '@testing-library/react';
import {beforeEach,afterEach,it,expect,vi} from 'vitest';
import AiChat from '../AiChat';
const {invokeMock}=vi.hoisted(()=>({invokeMock:vi.fn()}));
vi.mock('@tauri-apps/api/core',()=>({invoke:(...args:unknown[])=>invokeMock(...args)}));
vi.mock('@tauri-apps/api/event',()=>({listen:()=>Promise.resolve(()=>{})}));
beforeEach(()=>{localStorage.clear();vi.clearAllMocks();invokeMock.mockImplementation(async(command:string)=>['get_knowledge_base_files','get_pending_project_reviews'].includes(command)?[]:command==='get_private_context_consent'?false:command==='get_provider'?'local':'');});
afterEach(cleanup);
it('inserts a sample once under StrictMode and never sends it automatically',async()=>{
    const consumed=vi.fn();const request={id:1,text:'Review this idea'};
    render(<StrictMode><AiChat projectDir="/public-fixture" onInjectCode={()=>{}} draftRequest={request} onDraftConsumed={consumed}/></StrictMode>);
    await waitFor(()=>expect(screen.getByPlaceholderText('Ask about your code...')).toHaveValue('Review this idea'));
    expect(consumed).toHaveBeenCalledExactlyOnceWith(1);
    expect(invokeMock.mock.calls.some(([command])=>command==='send_ai_message')).toBe(false);
});
it('appends to an existing draft and does not repeat a consumed request',async()=>{
    const props={projectDir:'/public-fixture',onInjectCode:()=>{}};
    const {rerender}=render(<AiChat {...props}/>);
    fireEvent.change(screen.getByPlaceholderText('Ask about your code...'),{target:{value:'My unsent draft'}});
    rerender(<AiChat {...props} draftRequest={{id:2,text:'New sample'}}/>);
    await waitFor(()=>expect(screen.getByPlaceholderText('Ask about your code...')).toHaveValue('My unsent draft\n\nNew sample'));
    rerender(<AiChat {...props} draftRequest={{id:2,text:'New sample'}}/>);
    expect(screen.getByPlaceholderText('Ask about your code...')).toHaveValue('My unsent draft\n\nNew sample');
    expect(invokeMock.mock.calls.some(([command])=>command==='send_ai_message')).toBe(false);
});

it('sends the selected Plus board as separate typed context without changing the user request',async()=>{
    const original=invokeMock.getMockImplementation()!;
    invokeMock.mockImplementation((command:string,...args:unknown[])=>command==='get_base_url'?Promise.resolve('http://127.0.0.1:11434/v1'):original(command,...args));
    render(<AiChat projectDir="." selectedBoard="miuaiplus" onInjectCode={()=>{}}/>);
    await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith('get_base_url'));
    const input=screen.getByPlaceholderText('Ask about your code...');
    fireEvent.change(input,{target:{value:'Display something on the screen'}});
    fireEvent.keyDown(input,{key:'Enter',shiftKey:false});
    await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith('send_ai_message',expect.objectContaining({
        selectedBoard:'miuaiplus',projectDir:'.',messages:[{role:'user',content:'Display something on the screen'}],
    })));
});
