import {readFileSync, existsSync, readdirSync} from "node:fs";
import {strict as assert} from "node:assert";
const read=p=>readFileSync(p,"utf8");
const pkg=JSON.parse(read("package.json"));
const lock=JSON.parse(read("package-lock.json"));
const tauri=JSON.parse(read("src-tauri/tauri.conf.json"));
assert.equal(pkg.version,"3.7.1");
assert.equal(lock.version,pkg.version);
assert.equal(lock.packages[""].version,pkg.version);
assert.equal(tauri.version,pkg.version);
assert.match(read("src-tauri/Cargo.toml"), /version = "3\.7\.1"/);
assert.match(read("src-tauri/Cargo.lock"), /name = "vibe-kidbright"\nversion = "3\.7\.1"/);
assert.equal(tauri.bundle.resources["../resources/knowledge_base"],"knowledge_base");
assert.ok(!read("src/App.tsx").includes('<option value="miuaiplus">'));
assert.ok(!read("src/App.tsx").includes('localStorage.getItem("vibe-selected-board")'));
assert.ok(!read("src-tauri/src/lib.rs").includes("esp_idf::list_adb_devices"));
assert.ok(!read("src-tauri/src/esp_idf.rs").includes("pub async fn start_adb_monitor"));
assert.ok(!existsSync("resources/knowledge_base/kidbright_uai.md"));
const sync=directory=>{
    for(const entry of readdirSync(`resources/knowledge_base/${directory}`,{withFileTypes:true})){
        const file=directory?`${directory}/${entry.name}`:entry.name;
        if(entry.isDirectory()){sync(file);continue;}
        const source=`knowledge_base/${file}`;
        assert.ok(existsSync(source),`Missing source ${source}`);
        assert.ok(readFileSync(source).equals(readFileSync(`resources/knowledge_base/${file}`)),`Unsynced ${file}`);
    }
};
sync("");
for (const workflow of ["build-windows.yml","build-macos.yml"]){
    const text=read(`.github/workflows/${workflow}`);
    assert.match(text,/branches: \[ hotfix\/3\.7\.1 \]/);
    assert.match(text,/make_latest: false/);
    assert.match(text,/target_commitish: \$\{\{ github.sha \}\}/);
}
console.log("3.7.1 versions, classic target, KB resources and isolated release workflows verified.");
