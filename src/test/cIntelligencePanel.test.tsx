import {beforeEach,describe,expect,it,vi} from "vitest";
import {act,fireEvent,render,screen,waitFor} from "@testing-library/react";
import CIntelligencePanel from "../CIntelligencePanel";
const invoke=vi.fn();vi.mock("@tauri-apps/api/core",()=>({invoke:(...args:unknown[])=>invoke(...args)}));
const answer={engine:'clangd',locations:[{path:'/a/main/main.c',line:4,character:2,end_line:4,end_character:5}],limitation:'opened files only',buffer_sha256:'hash',compile_commands_sha256:'cdb'};
const props={projectDir:'/a',filePath:'/a/main/main.c',buffers:[{path:'/a/main/main.c',text:'unsaved'}],getPosition:()=>({lineNumber:2,column:5}),onNavigate:vi.fn()};
describe('local C intelligence',()=>{
    beforeEach(()=>{vi.clearAllMocks();localStorage.clear();invoke.mockResolvedValue(answer);});
    it('sends unsaved text locally with clangd disabled until explicitly enabled',async()=>{
        render(<CIntelligencePanel {...props}/>);fireEvent.click(screen.getByText('C intelligence'));fireEvent.click(screen.getByText('Find definition'));
        await screen.findByText('clangd semantic results');
        expect(invoke.mock.calls[0][1].request).toEqual({file_path:props.filePath,buffers:props.buffers,line:1,character:4,operation:'definition',clangd_path:''});
        fireEvent.click(screen.getByText('main/main.c:5:3'));expect(props.onNavigate).toHaveBeenCalledWith('/a/main/main.c',5,3);
    });
    it('cancels old workspace requests and discards their late results',async()=>{
        let complete!:(value:unknown)=>void;invoke.mockImplementation((cmd:string)=>cmd==='query_c_intelligence'?new Promise(resolve=>{complete=resolve;}):Promise.resolve());
        const view=render(<CIntelligencePanel {...props}/>);fireEvent.click(screen.getByText('C intelligence'));fireEvent.click(screen.getByText('Find references'));
        view.rerender(<CIntelligencePanel {...props} projectDir='/b' filePath='/b/main/main.c' buffers={[]}/>);
        await waitFor(()=>expect(invoke).toHaveBeenCalledWith('cancel_c_intelligence',expect.objectContaining({requestId:expect.any(String)})));
        await act(async()=>complete(answer));expect(screen.queryByText('clangd semantic results')).not.toBeInTheDocument();
    });
    it('stops a pending query without accepting its eventual result',async()=>{
        let complete!:(value:unknown)=>void;invoke.mockImplementation((cmd:string)=>cmd==='query_c_intelligence'?new Promise(resolve=>{complete=resolve;}):Promise.resolve());
        render(<CIntelligencePanel {...props}/>);fireEvent.click(screen.getByText('C intelligence'));fireEvent.click(screen.getByText('Find definition'));fireEvent.click(screen.getByText('Stop C intelligence'));
        await act(async()=>complete(answer));await screen.findByText('C intelligence cancelled');expect(screen.queryByText('clangd semantic results')).not.toBeInTheDocument();
    });
    it('shows syntax fallback as candidates rather than semantic references',async()=>{
        invoke.mockResolvedValue({...answer,engine:'tree_sitter_syntax',limitation:'No exact compile command'});
        render(<CIntelligencePanel {...props}/>);fireEvent.click(screen.getByText('C intelligence'));fireEvent.click(screen.getByText('Find references'));
        await screen.findByText('Tree-sitter syntax candidates');expect(screen.getByText('No exact compile command')).toBeInTheDocument();
    });
    it('requires a fresh explicit enable even when an executable path was saved',async()=>{
        localStorage.setItem('vibe-clangd-path','C:/trusted/clangd.exe');
        render(<CIntelligencePanel {...props}/>);fireEvent.click(screen.getByText('C intelligence'));
        expect(screen.getByRole('checkbox')).not.toBeChecked();
        fireEvent.click(screen.getByRole('checkbox'));fireEvent.click(screen.getByText('Find definition'));
        await screen.findByText('clangd semantic results');expect(invoke.mock.calls[0][1].request.clangd_path).toBe('C:/trusted/clangd.exe');
    });
});
