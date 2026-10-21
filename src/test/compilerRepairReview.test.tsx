import {act,fireEvent,render,screen,waitFor} from "@testing-library/react";
import {beforeEach,describe,expect,it,vi} from "vitest";
import CompilerRepairReview from "../CompilerRepairReview";
const {invokeMock,handlers}=vi.hoisted(()=>({invokeMock:vi.fn(),handlers:new Map<string,()=>void>()}));
vi.mock("@tauri-apps/api/core",()=>({invoke:(...a:unknown[])=>invokeMock(...a)}));
vi.mock("@tauri-apps/api/event",()=>({listen:(n:string,h:()=>void)=>{handlers.set(n,h);return Promise.resolve(()=>handlers.delete(n));}}));
const fixture=()=>({revision:"revision-1",session:{root:"/project",phase:"awaiting_approval",attempts:0,max_attempts:3,last_error:null,diagnostics:[],changes:[{path:"/project/main/main.c",before:"broken",proposed:"fixed"}]}});
const props={projectDir:"/project",isBusy:false,hasUnsavedChanges:false,onRetry:vi.fn()};
describe("Compiler repair approval",()=>{
    beforeEach(()=>{vi.clearAllMocks();handlers.clear();invokeMock.mockResolvedValue(fixture());});
    it("requires an explicit checkbox and sends the current revision for one attempt",async()=>{
        render(<CompilerRepairReview {...props}/>);const apply=await screen.findByText("Apply this patch & verify build");expect(apply).toBeDisabled();
        fireEvent.click(screen.getByRole("checkbox"));fireEvent.click(apply);
        await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("apply_compiler_repair",{projectDir:"/project",expectedRevision:"revision-1"}));
    });
    it("clears approval whenever a new revision arrives",async()=>{
        render(<CompilerRepairReview {...props}/>);await screen.findByRole("checkbox");fireEvent.click(screen.getByRole("checkbox"));
        const next=fixture();next.revision="revision-2";invokeMock.mockResolvedValue(next);
        await act(async()=>{handlers.get("compiler-repair-changed")?.();});
        await waitFor(()=>expect(screen.getByRole("checkbox")).not.toBeChecked());expect(screen.getByText("Apply this patch & verify build")).toBeDisabled();
        invokeMock.mockResolvedValue(fixture());
        await act(async()=>{handlers.get("compiler-repair-changed")?.();});
        expect(screen.getByRole("checkbox")).not.toBeChecked();
        expect(screen.getByText("Apply this patch & verify build")).toBeDisabled();
    });
    it("consumes approval even when apply fails and the same revision is returned",async()=>{
        invokeMock.mockImplementation((command:string)=>command==="apply_compiler_repair"?Promise.reject(new Error("temporary failure")):Promise.resolve(fixture()));
        render(<CompilerRepairReview {...props}/>);fireEvent.click(await screen.findByRole("checkbox"));
        fireEvent.click(screen.getByText("Apply this patch & verify build"));
        await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("apply_compiler_repair",{projectDir:"/project",expectedRevision:"revision-1"}));
        await waitFor(()=>expect(screen.getByRole("checkbox")).not.toBeDisabled());
        expect(screen.getByRole("checkbox")).not.toBeChecked();
        expect(screen.getByText("Apply this patch & verify build")).toBeDisabled();
        fireEvent.click(screen.getByText("Apply this patch & verify build"));
        expect(invokeMock.mock.calls.filter(([command])=>command==="apply_compiler_repair")).toHaveLength(1);
    });
    it("does not transfer approval to another project with the same revision label",async()=>{
        const {rerender}=render(<CompilerRepairReview {...props}/>);
        fireEvent.click(await screen.findByRole("checkbox"));
        expect(screen.getByRole("checkbox")).toBeChecked();
        rerender(<CompilerRepairReview {...props} projectDir="/other"/>);
        await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("get_compiler_repair",{projectDir:"/other"}));
        await screen.findByRole("checkbox");
        expect(screen.getByRole("checkbox")).not.toBeChecked();
        expect(screen.getByText("Apply this patch & verify build")).toBeDisabled();
    });
    it("blocks applied patches and rollback while drafts are unsaved",async()=>{
        render(<CompilerRepairReview {...props} hasUnsavedChanges/>);await screen.findByRole("checkbox");
        expect(screen.getByRole("checkbox")).toBeDisabled();expect(screen.getByText("Undo this attempt")).toBeDisabled();
    });
    it("still permits stopping verification when an editor draft is dirty",async()=>{
        const state=fixture();state.session.phase="building";invokeMock.mockResolvedValue(state);
        render(<CompilerRepairReview {...props} hasUnsavedChanges isBusy/>);
        const stop=await screen.findByText("Stop verification");expect(stop).not.toBeDisabled();fireEvent.click(stop);
        await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("cancel_compiler_repair",{projectDir:"/project",expectedRevision:"revision-1"}));
    });
    it("can stop a native build while the apply request is still waiting",async()=>{
        let resolveApply:(value:unknown)=>void=()=>{};let state=fixture();
        invokeMock.mockImplementation((cmd:string)=>{
            if(cmd==="get_compiler_repair")return Promise.resolve(state);
            if(cmd==="apply_compiler_repair")return new Promise(resolve=>{resolveApply=resolve;});
            return Promise.resolve({...state,session:{...state.session,phase:"cancelled"}});
        });
        render(<CompilerRepairReview {...props}/>);fireEvent.click(await screen.findByRole("checkbox"));fireEvent.click(screen.getByText("Apply this patch & verify build"));
        state={...state,revision:"building-revision",session:{...state.session,phase:"building"}};
        await act(async()=>{handlers.get("compiler-repair-changed")?.();});
        fireEvent.click(await screen.findByText("Stop verification"));
        await waitFor(()=>expect(invokeMock).toHaveBeenCalledWith("cancel_compiler_repair",{projectDir:"/project",expectedRevision:"building-revision"}));
        await act(async()=>resolveApply({...state,session:{...state.session,phase:"cancelled"}}));
    });
});
