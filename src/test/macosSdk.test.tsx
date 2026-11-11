import {act,cleanup,fireEvent,render,screen,waitFor} from "@testing-library/react";
import {afterEach,describe,expect,it,vi} from "vitest";
import MacosToolchainSetup from "../MacosToolchainSetup";
const {invokeMock}=vi.hoisted(()=>({invokeMock:vi.fn()}));
vi.mock("@tauri-apps/api/core",()=>({invoke:invokeMock}));
afterEach(()=>{cleanup();vi.resetAllMocks();});
describe("3.7.1 native Mac SDK repair",()=>{
  it("keeps missing Python unready and checks the native SDK after setup",async()=>{
    const ready=vi.fn();
    invokeMock.mockResolvedValueOnce({status:"not_installed",error:"Native Python environment is missing"})
      .mockResolvedValueOnce({status:"ready"}).mockResolvedValueOnce({status:"ready"});
    render(<MacosToolchainSetup onReady={ready}/>);
    expect(await screen.findByRole("alert")).toHaveTextContent("Native Python environment is missing");
    expect(ready).not.toHaveBeenCalled();
    await act(async()=>{fireEvent.click(screen.getByRole("button",{name:"Check Native SDK"}));});
    await waitFor(()=>expect(ready).toHaveBeenCalledTimes(1));
    expect(invokeMock.mock.calls.map(([command])=>command)).toEqual(["check_toolchain","select_native_macos_sdk","check_toolchain"]);
    expect(invokeMock).not.toHaveBeenCalledWith("download_toolchain",expect.anything());
  });
});
