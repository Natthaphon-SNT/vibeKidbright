// Exercises the real built UI and native browser dialog. The caller supplies
// its explicit Tauri fixture; the guide itself needs no native/provider calls.
import {mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';

export async function probeBoardGuide(page, root) {
    const assert=(condition,message)=>{if(!condition)throw new Error(message);};
    const learning=page.locator('.learning-workspace');
    await page.getByRole('button',{name:'ห้องเรียน AI',exact:true}).click();
    await learning.getByRole('button',{name:'TH',exact:true}).click();
    await learning.getByRole('button',{name:'μAI Plus',exact:true}).click();
    const trigger=learning.locator('.lw-hero').getByRole('button',{name:'คู่มือเตรียมบอร์ด',exact:true});
    const saved=await page.evaluate(()=>window.__workflowProbe.state());
    const beforeCommands=await page.evaluate(()=>window.__workflowProbe.commands.length);
    const screenshotRoot=resolve(root,'docs/previews');
    await mkdir(screenshotRoot,{recursive:true});
    const screenshots=[];
    const capture=async name=>{
        await page.locator('dialog[open] .lw-guide-reader').evaluate(node=>{node.scrollTop=0;});
        const path=resolve(screenshotRoot,name);await page.screenshot({path});screenshots.push('docs/previews/'+name);
    };
    await page.setViewportSize({width:1280,height:800});
    await trigger.click();
    const dialog=page.getByRole('dialog',{name:'คู่มือเตรียมบอร์ด',exact:true});
    await dialog.getByRole('heading',{name:'ตรวจ USB network บน Windows',exact:true}).waitFor();
    assert(await dialog.evaluate(node=>node.matches(':modal')),'Guide did not open in the browser top layer');
    assert(await dialog.evaluate(node=>node.contains(document.activeElement)),'Guide did not receive focus');
    await page.evaluate(()=>document.querySelector('nav[aria-label="Workspace views"] button').focus());
    assert(await dialog.evaluate(node=>node.contains(document.activeElement)),'Focus escaped the modal into the app');
    for(let i=0;i<5;i++)await page.keyboard.press('Tab');
    assert(await dialog.evaluate(node=>node.contains(document.activeElement)),'Keyboard focus escaped the guide');
    const desktop=await dialog.boundingBox();
    assert(desktop&&desktop.x>=0&&desktop.y>=0&&desktop.x+desktop.width<=1281&&desktop.y+desktop.height<=801,'Desktop guide is outside the viewport');
    await capture('board-guide-th-plus.png');
    await page.keyboard.press('Escape');
    await dialog.waitFor({state:'detached'});
    assert(await trigger.evaluate(node=>node===document.activeElement),'Escape did not restore focus to the opener');

    await learning.getByRole('button',{name:'EN',exact:true}).click();
    const cases=[
        ['KidBright32',"Check the board's serial port",'Check USB networking on Windows'],
        ['μAI','Identify the original V831 μAI','Check USB networking on Windows'],
        ['μAI Plus','Check USB networking on Windows',"Check the board's serial port"],
    ];
    for(const [board,expected,excluded] of cases){
        await learning.getByRole('button',{name:board,exact:true}).click();
        await learning.locator('.lw-guide-card').getByRole('button',{name:'Board preparation guide',exact:true}).click();
        const guide=page.getByRole('dialog',{name:'Board preparation guide',exact:true});
        await guide.getByRole('heading',{name:expected,exact:true}).waitFor();
        assert(await guide.getByText(excluded,{exact:true}).count()===0,'Wrong board instructions: '+board);
        const problem=guide.locator('summary').first();
        await problem.click();
        assert(await problem.evaluate(node=>node.parentElement.open),'Troubleshooting did not expand');
        if(board==='μAI')await capture('board-guide-en-mai.png');
        await guide.getByRole('button',{name:'Back to lesson',exact:true}).click();
        await guide.waitFor({state:'detached'});
    }

    await page.setViewportSize({width:800,height:700});
    await page.evaluate(()=>document.documentElement.setAttribute('data-theme','dark'));
    await learning.locator('.lw-hero').getByRole('button',{name:'Board preparation guide',exact:true}).click();
    const compact=page.getByRole('dialog',{name:'Board preparation guide',exact:true});
    const bounds=await compact.boundingBox();
    assert(bounds&&bounds.x>=0&&bounds.y>=0&&bounds.x+bounds.width<=801&&bounds.y+bounds.height<=701,'Compact guide is outside the viewport');
    assert(await compact.locator('.lw-guide-reader').evaluate(node=>node.scrollHeight>node.clientHeight),'Compact reader is not scrollable');
    await capture('board-guide-compact-dark.png');
    await page.mouse.click(3,3);
    await compact.waitFor({state:'detached'});
    await page.evaluate(()=>document.documentElement.setAttribute('data-theme','light'));
    await page.setViewportSize({width:1440,height:1000});

    await learning.getByRole('button',{name:'KidBright32',exact:true}).click();
    await learning.locator('.lw-hero').getByRole('button',{name:'Board preparation guide',exact:true}).click();
    await page.getByRole('dialog').getByRole('button',{name:'Open code workspace',exact:true}).click();
    await page.getByRole('dialog').waitFor({state:'detached'});
    assert(await page.getByRole('button',{name:'เขียนโปรแกรม',exact:true}).getAttribute('aria-pressed')==='true','Guide did not navigate to the real editor');
    const after=await page.evaluate(()=>({state:window.__workflowProbe.state(),commands:window.__workflowProbe.commands}));
    assert(JSON.stringify(saved)===JSON.stringify(after.state),'Guide modified project or review state');
    const mutations=after.commands.slice(beforeCommands).filter(({cmd})=>/^(send_ai_message|accept_diff|write_project_file|build_project|flash_project|install_runtime|connect_adb)$/.test(cmd));
    assert(mutations.length===0,'Reading the guide triggered a mutation');
    return {native_dialog_top_layer:true,thai_and_english:true,board_families:3,both_entry_points:true,escape_focus_restored:true,
        background_focus_blocked:true,troubleshooting_expands:true,backdrop_close:true,editor_navigation:true,
        desktop_and_compact_viewports:true,dark_theme:true,source_and_review_unchanged:true,mutation_commands:0,screenshots,
        scope:'Actual production React/CSS and browser dialog under production CSP; native startup/provider replies are fixtures. No hardware, installed desktop IPC or installer lifecycle claim.'};
}
