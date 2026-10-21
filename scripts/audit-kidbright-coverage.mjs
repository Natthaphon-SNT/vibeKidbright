import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Read and parse reference sources as data. Never import or execute their code.
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidencePath = "docs/research/kidbright-2026-10-15-evidence.json";
const indexPath = "docs/research/kidbright-static-source-index-2026-10-16.json";
const outputPath = "docs/research/kidbright-coverage-2026-10-16.json";
const reportPath = "docs/KIDBRIGHT_BLOCK_COVERAGE_2026-10-16.md";
const pilotPath = "docs/research/kidbright-pilot-2026-10-16-validation.json";
const expansionPath = "docs/research/kidbright-expansion-2026-10-17-validation.json";
const args = process.argv.slice(2);
const sha = (value) => createHash("sha256").update(value).digest("hex");
const read = (path) => readFileSync(join(root, path), "utf8");
const json = (path) => JSON.parse(read(path));
const sorted = (items) => [...new Set(items)].sort((a, b) => a.localeCompare(b, "en"));
const option = (name) => {
  const index = args.indexOf(name);
  if (index < 0) return undefined;
  if (!args[index + 1] || args[index + 1].startsWith("--")) throw new Error(`${name} requires a cache root`);
  return resolve(args[index + 1]);
};
const evidence = json(evidencePath);
const pilot = json(pilotPath);
const expansion = json(expansionPath);
const originalSources = [
  ...evidence.archive_sources.map((source) => ({ ...source, scope: "archive" })),
  ...evidence.web_sources.filter((source) => source.status === 200).map((source) => ({ ...source, scope: "web" })),
];
const generatorHelpers = new Set(["init", "finish", "scrubNakedValue", "quote_", "scrub_", "getAdjusted", "lists.getIndex_", "text.getIndex_", "resetTaskNumber"]);
const isMutator = (id) => /^(?:lists_create_with_(?:container|item)|text_create_join_(?:container|item)|controls_if_(?:if|else|elseif)|procedures_mutator(?:arg|container))$/.test(id);
const isVerticalAsset = (item) => item.path.endsWith("blocks_compressed_vertical.js");
const isKidBrightCustom = (item) => item.path.includes("/blockly/kidbright/") || item.path.startsWith("blockly/kidbright/") || item.path.startsWith("kbide/plugins/");

function sourceBytes(cache, source) {
  const file = join(cache, source.scope, source.path);
  const bytes = readFileSync(file);
  if (sha(bytes) !== source.sha256 || bytes.length !== source.bytes) {
    throw new Error(`Reference hash or size differs: ${source.scope}:${source.path}`);
  }
  return bytes;
}

async function captureSourceIndex(cache) {
  const ts = await import("typescript");
  const assignments = [];
  const parseDiagnostics = [];
  const sources = [];
  function chain(node) {
    if (ts.isIdentifier(node)) return [node.text];
    if (ts.isPropertyAccessExpression(node)) {
      const left = chain(node.expression);
      return left && [...left, node.name.text];
    }
    if (ts.isElementAccessExpression(node) && ts.isStringLiteralLike(node.argumentExpression)) {
      const left = chain(node.expression);
      return left && [...left, node.argumentExpression.text];
    }
    return undefined;
  }
  const propertyName = (node) => (ts.isIdentifier(node) || ts.isStringLiteralLike(node)) ? node.text : undefined;
  for (const source of originalSources.filter((source) => source.path.endsWith(".js"))) {
    const bytes = sourceBytes(cache, source);
    const file = ts.createSourceFile(source.path, bytes.toString("utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    sources.push({ scope: source.scope, path: source.path, sha256: source.sha256, bytes: source.bytes });
    for (const diagnostic of file.parseDiagnostics) {
      parseDiagnostics.push({ scope: source.scope, path: source.path, line: file.getLineAndCharacterOfPosition(diagnostic.start ?? 0).line + 1, message: ts.flattenDiagnosticMessageText(diagnostic.messageText, " ") });
    }
    function add(node, kind, id, extra = {}) {
      assignments.push({ scope: source.scope, kind, type: id, path: source.path, line: file.getLineAndCharacterOfPosition(node.getStart(file)).line + 1, ...extra });
    }
    function visit(node) {
      if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken) {
        const name = chain(node.left);
        if (name?.[0] === "Blockly" && name.length >= 3) {
          const id = name.slice(2).join(".");
          if (name[1] === "Blocks" && name.length === 3 && ts.isObjectLiteralExpression(node.right)) {
            const init = node.right.properties.some((prop) => prop.name && propertyName(prop.name) === "init");
            if (init) add(node, "block", id, { declaration: "assignment", mutator_only: isMutator(id) });
          }
          if (name[1] === "JavaScript" && (ts.isFunctionExpression(node.right) || ts.isArrowFunction(node.right))) {
            add(node, "generator", id, { declaration: "function", helper_only: generatorHelpers.has(id) });
          }
          const target = chain(node.right);
          if (name[1] === "JavaScript" && target?.[0] === "Blockly" && target?.[1] === "JavaScript") {
            add(node, "generator", id, { declaration: "alias", alias_target: target.slice(2).join("."), helper_only: generatorHelpers.has(id) });
          }
        }
      }
      if (ts.isCallExpression(node) && chain(node.expression)?.join(".") === "Blockly.defineBlocksWithJsonArray") {
        const array = node.arguments[0];
        if (array && ts.isArrayLiteralExpression(array)) {
          for (const block of array.elements) {
            if (!ts.isObjectLiteralExpression(block)) continue;
            for (const prop of block.properties) {
              if (ts.isPropertyAssignment(prop) && propertyName(prop.name) === "type" && ts.isStringLiteralLike(prop.initializer)) {
                add(prop, "block", prop.initializer.text, { declaration: "json-array", mutator_only: isMutator(prop.initializer.text) });
              }
            }
          }
        }
      }
      ts.forEachChild(node, visit);
    }
    visit(file);
  }
  if (parseDiagnostics.length) throw new Error(`Reference JS has ${parseDiagnostics.length} parse diagnostics; inspect before using AST counts: ${JSON.stringify(parseDiagnostics.slice(0, 4))}`);
  const result = {
    schema_version: 1,
    capture_date: "2026-10-16",
    evidence_path: evidencePath,
    evidence_sha256: sha(readFileSync(join(root, evidencePath))),
    method: "TypeScript JavaScript AST parse, no source execution. Only direct static Blockly block definitions, JSON-array definitions, function generators and aliases. Comments are ignored. Presence does not prove loading or firmware compatibility.",
    parser: { package: "typescript", version: ts.version },
    sources,
    parse_diagnostics: parseDiagnostics,
    assignments,
  };
  writeFileSync(join(root, indexPath), `${JSON.stringify(result, null, 2)}\n`);
  console.log(`Captured ${assignments.length} active static assignments from ${sources.length} hash-verified JS files`);
}

const captureCache = option("--capture-source");
if (captureCache) await captureSourceIndex(captureCache);
if (!existsSync(join(root, indexPath))) throw new Error(`Missing ${indexPath}; capture it with --capture-source <research-cache>`);
const sourceIndex = json(indexPath);
if (sourceIndex.evidence_sha256 !== sha(readFileSync(join(root, evidencePath)))) throw new Error("Original evidence changed; review and recapture the source index");
if (sourceIndex.parse_diagnostics.length) throw new Error("Source index has unresolved parse diagnostics");

const verifyCache = option("--verify-source");
if (verifyCache) {
  for (const source of originalSources) sourceBytes(verifyCache, source);
  const home = readFileSync(join(verifyCache, "web", "home.html"));
  if (sha(home) !== evidence.web_home.sha256) throw new Error("Web home hash differs");
  console.log(`Verified ${originalSources.length} reference files and web home`);
}

function listFiles(directory) {
  return readdirSync(join(root, directory), { withFileTypes: true }).flatMap((entry) => {
    const path = `${directory}/${entry.name}`;
    return entry.isDirectory() ? listFiles(path) : [path];
  }).sort();
}
const ruleFiles = [...listFiles("knowledge_base").filter((file) => /\.(md|txt)$/.test(file)), "src-tauri/src/ai/system_prompt.txt"];
const exampleFiles = listFiles("knowledge_base").filter((file) => /\.(c|h)$/.test(file));
const supportFiles = [...ruleFiles, ...exampleFiles, "src-tauri/tests/kidbright_core_test.c", "src-tauri/src/ai/compatibility.rs"];
const textByFile = new Map(supportFiles.map((file) => [file, read(file)]));
const sourceRef = (item) => `${item.scope === "archive" ? "K" : "W"}:${item.path}:${item.line}`;
const reference = (path, needle) => {
  const text = textByFile.get(path) ?? read(path);
  const offset = text.indexOf(needle);
  if (offset < 0) throw new Error(`Reviewed implementation marker is missing: ${path} ${needle}`);
  return { path, line: text.slice(0, offset).split("\n").length };
};
const coreFile = "knowledge_base/compatibility/kb_core.c";
const testFile = "src-tauri/tests/kidbright_core_test.c";
function component(id, symbols, scope) {
  const currentHash = sha(readFileSync(join(root, coreFile)));
  const recorded = pilot.reference_core.find((item) => item.path === coreFile);
  const currentSourceMatches = currentHash === recorded?.sha256;
  return { id, symbols, implementation: symbols.map((symbol) => reference(coreFile, `${symbol}(`)), tests: symbols.map((symbol) => reference(testFile, `${symbol}(`)), scope, status: currentSourceMatches ? "recorded-host-tested-component-only" : "recorded-tests-stale-for-current-source", current_source_sha256: currentHash, current_source_matches_recorded_validation: currentSourceMatches, original_generator_differential_tested: false, physical_tested: false };
}
const components = {
  delay: component("delay", ["kb_delay_ms"], "Nonnegative finite seconds -> truncated integer milliseconds; scheduling and arbitrary block composition are not certified"),
  lm73: component("lm73", ["kb_lm73_celsius"], "Signed MSB-first LM73 raw /128 decoding; physical sensor/bus behavior is not certified"),
  pid: component("pid", ["kb_pid_step"], "Time-based PID with separate state, I ±60/output ±100; first-sample and invalid-dt repairs and double precision differ explicitly"),
  sma: component("sma", ["kb_sma_step"], "Warm-up and rolling SMA; windows 1..50 only; window changes reset state; no EMA implementation"),
  joystick: component("joystick", ["kb_joystick_init", "kb_joystick_read"], "One calibrated numeric axis, descending endpoints and strict ±500 dead zone; no ADC driver, two-axis dispatcher or calibration UI"),
  ascii: component("ascii", ["kb_ascii_format", "kb_ascii_parse"], "Legacy numeric %f payload codec only; no ESP-NOW transport, callbacks, peer management or string API"),
};
const componentMap = new Map([
  ["basic_delay", ["delay"]], ["ms_delay", ["delay"]], ["sensor_lm73", ["lm73"]],
  ["pid_con.run", ["pid"]], ["pid_con.run2", ["pid"]], ["pid_con.run3", ["pid"]],
  ["filter.sma", ["sma"]], ["mjoystick.get_position", ["joystick"]],
  ["mjoystick.calibrate_set_x", ["joystick"]], ["mjoystick.calibrate_set_y", ["joystick"]],
  ["esp_now.send", ["ascii"]], ["esp_now.send2", ["ascii"]], ["esp_now.read_number", ["ascii"]],
]);
const recipeMap = new Map();
function mapRecipe(ids, recipe, scope) {
  for (const id of ids) recipeMap.set(id, [...(recipeMap.get(id) ?? []), { recipe, scope }]);
}
mapRecipe(["basic_forever", "logic_sw1_pressed", "logic_sw1_released", "sensor_switch1", "wait_sw1_pressed", "wait_sw1_released"], "button_buzzer", "Fixed S1 active-low press/beep/release loop only; not a general block adapter");
mapRecipe(["music_note"], "button_buzzer", "Only A4 440 Hz, 500 ms, fixed 50% LEDC duty; not the complete note/scale/tempo/volume API");
mapRecipe(["basic_delay"], "button_buzzer", "Fixed 0.5 s note duration plus 40 ms polling");
mapRecipe(["basic_delay", "basic_forever", "sensor_lm73"], "temperature_serial", "Fixed LM73 combined transaction, /128 decode, serial log every 500 ms");

function category(id, definitions) {
  const path = definitions.find((item) => item.scope === "archive")?.path ?? definitions[0]?.path ?? "";
  if (definitions.length && definitions.every(isVerticalAsset)) return "Bundled vertical editor assets (loading unverified)";
  if (isMutator(id)) return "Blockly editor mutators";
  if (/^(?:basic_led|logic_led|wait_led|oled_|lcd_|ws2812|blink\.)/.test(id)) return "Displays and lighting";
  if (/^(?:logic_sw|wait_sw|sensor_switch|output_|input_|usbsw_|mcp23|pulse_|spec_ios|specialoutput_|get_gpio$|set_gpio$)/.test(id)) return "Digital I/O and waits";
  if (/^(?:music_)/.test(id)) return "Music and audio";
  if (/^(?:sensor_|rd_analog|mpu6050|ads1115|adsws1|hpma|sht21|sht31|si7021|tsl2561|ds18b20|ultrasonic\.data)/.test(id)) return "Sensors and analog input";
  if (/^rtc_/.test(id)) return "RTC and clock";
  if (/^(?:pid_con|filter\.|mjoystick|joystick_|joystick\.|pwm_|quad_encoder|drv883[03]|myservo|ikb1_)/.test(id)) return "Control, motors and expansion boards";
  if (/^(?:esp_now|mqtt_|netpie_|bt_|ble_uart|linenotify|ifttt|FIR\.|gauge_|feed_|push_notification)/.test(id)) return "Networking, cloud and IoT";
  if (/^(?:serial_|ser_print)/.test(id)) return "Serial and plotting";
  if (/^(?:math_variables|string_variables|variables_)/.test(id)) return "Variables";
  if (/^(?:math|convert_ra_de|spec_math_number)/.test(id)) return "Math and constants";
  if (/^(?:logic_|controls_if)/.test(id)) return "Logic and conditions";
  if (/^(?:basic_delay|ms_delay|basic_forever|loop_|controls_|advance_task)/.test(id)) return "Loops, delays and tasks";
  if (/^(?:function$|call_function$|procedures_)/.test(id)) return "Functions and procedures";
  if (/^(?:basic_string|string_|text|number-to-str)/.test(id)) return "Text and conversion";
  if (/^(?:list|lists_)/.test(id)) return "Lists";
  if (/^colour_/.test(id)) return "Colors";
  if (path.includes("ultrasonic") || id.startsWith("ultrasonic.")) return "Ultrasonic auxiliary blocks";
  return "Generator helpers or unclassified IDs";
}
const rawAssignments = evidence.block_assignments;
const active = sourceIndex.assignments;
const indexIds = sorted([
  ...rawAssignments.map((item) => item.type),
  ...evidence.builtin_generator_assignments.filter((item) => !generatorHelpers.has(item.type)).map((item) => item.type),
  ...active.filter((item) => !item.helper_only).map((item) => item.type),
  ...evidence.toolbox.flatMap((item) => item.types),
  ...evidence.sample_block_types.all_types,
]);
const exactMention = (id) => {
  const escaped = id.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`(^|[^A-Za-z0-9_.-])${escaped}(?=$|[^A-Za-z0-9_.-])`);
  return ruleFiles.flatMap((path) => textByFile.get(path).split(/\r?\n/).flatMap((line, index) => pattern.test(line) ? [{ path, line: index + 1 }] : []));
};
const blocks = indexIds.map((id) => {
  const definitions = active.filter((item) => item.type === id && item.kind === "block");
  const generators = active.filter((item) => item.type === id && item.kind === "generator" && !item.helper_only);
  const originalAssignments = rawAssignments.filter((item) => item.type === id);
  const helper = generatorHelpers.has(id);
  const mutator = isMutator(id);
  const portable = componentMap.get(id) ?? [];
  const recipes = (recipeMap.get(id) ?? []).map((mapping) => {
    const path = `knowledge_base/compatibility/${mapping.recipe}.c`;
    const build = pilot.firmware_builds.find((build) => build.example === mapping.recipe);
    if (!build) throw new Error(`No recorded firmware evidence for ${mapping.recipe}`);
    const currentHash = sha(readFileSync(join(root, path)));
    const matches = currentHash === build.main_source.sha256;
    return { ...mapping, source: { path, sha256: currentHash }, recorded_build: { status: matches && build.exit_code === 0 ? "passed-for-current-source" : "stale-or-failed", board_profile: build.board_profile, sdk_version: build.sdk_version, target: build.target, evidence: pilotPath }, physical_tested: false };
  });
  const kbMentions = exactMention(id);
  const lexicalAmbiguity = ["function", "text"].includes(id);
  const group = category(id, definitions);
  return {
    id, category: group,
    record_kind: helper ? "generator-helper" : mutator ? "editor-mutator" : definitions.length ? "block-definition" : "generator-or-toolbox-or-sample-only",
    archive: { raw_definition_mentions: originalAssignments.filter((item) => item.scope === "archive" && item.kind === "block"), definitions: definitions.filter((item) => item.scope === "archive"), generators: generators.filter((item) => item.scope === "archive") },
    web: { raw_definition_mentions: originalAssignments.filter((item) => item.scope === "web" && item.kind === "block"), definitions: definitions.filter((item) => item.scope === "web"), generators: generators.filter((item) => item.scope === "web") },
    toolbox_categories: evidence.toolbox.filter((item) => item.types.includes(id)).map((item) => item.name),
    sample_files: evidence.sample_block_types.samples.filter((item) => item.types.includes(id)).map((item) => item.name),
    active_in_samples: evidence.sample_block_types.active_types.includes(id),
    kb_exact_id_mentions: { status: "lexical-candidate-only-not-semantic-proof", generic_word_ambiguous: lexicalAmbiguity, matches: kbMentions },
    portable_component_ids: portable,
    firmware_recipe_subsets: recipes,
    full_block_adapter: "not-implemented-or-not-verified",
    physical_equivalence: "not-tested",
    status: helper || mutator ? "editor-or-generator-infrastructure" : portable.length ? "scoped-host-component" : recipes.length ? "fixed-recipe-subset" : "no-verified-block-port",
    gap: helper || mutator ? "Infrastructure IDs are not normal runtime features" : portable.length || recipes.length ? "Only the explicitly listed subset is evidenced; no general Blockly import/composition, end-to-end model evaluation or original/physical differential proof" : "A matching natural-language generation rule, validated board resource contract, ported API/semantics and relevant executable tests are not established by this audit",
  };
});

const count = (predicate) => blocks.filter(predicate).length;
const summaryByScope = (scope) => ({
  raw_custom_definition_assignments: rawAssignments.filter((item) => item.scope === scope && item.kind === "block").length,
  raw_custom_unique_definition_ids: new Set(rawAssignments.filter((item) => item.scope === scope && item.kind === "block").map((item) => item.type)).size,
  active_custom_unique_definition_ids: new Set(active.filter((item) => item.scope === scope && item.kind === "block" && isKidBrightCustom(item)).map((item) => item.type)).size,
  active_builtin_unique_definition_ids: new Set(active.filter((item) => item.scope === scope && item.kind === "block" && item.path.endsWith("blocks_compressed.js")).map((item) => item.type)).size,
  custom_builtin_overlapping_ids: blocks.filter((block) => block[scope].definitions.some(isKidBrightCustom) && block[scope].definitions.some((item) => item.path.endsWith("blocks_compressed.js"))).map((block) => block.id),
  active_bundled_vertical_asset_unique_ids: new Set(active.filter((item) => item.scope === scope && item.kind === "block" && isVerticalAsset(item)).map((item) => item.type)).size,
  total_active_unique_definition_ids: count((block) => block[scope].definitions.some((item) => !isVerticalAsset(item))),
  total_observed_asset_unique_definition_ids: count((block) => block[scope].definitions.length > 0),
  definitions_without_observed_generator: blocks.filter((block) => block[scope].definitions.some((item) => !isVerticalAsset(item)) && !block[scope].generators.length).map((block) => block.id),
  raw_definition_mentions_without_active_assignment: rawAssignments.filter((item) => item.scope === scope && item.kind === "block" && !active.some((candidate) => candidate.scope === item.scope && candidate.kind === item.kind && candidate.type === item.type && candidate.path === item.path && candidate.line === item.line)),
  raw_generator_mentions_without_active_assignment: rawAssignments.filter((item) => item.scope === scope && item.kind === "generator" && !active.some((candidate) => candidate.scope === item.scope && candidate.kind === item.kind && candidate.type === item.type && candidate.path === item.path && candidate.line === item.line)),
  builtin_generator_aliases: active.filter((item) => item.scope === scope && item.kind === "generator" && item.declaration === "alias" && item.path.endsWith("javascript_compressed.js")),
});
const modules = sorted(active.filter((item) => item.scope === "archive" && item.kind === "block" && isKidBrightCustom(item)).map((item) => item.path)).map((path) => {
  const ids = sorted(active.filter((item) => item.scope === "archive" && item.kind === "block" && item.path === path).map((item) => item.type));
  return { path, origin: path.startsWith("kbide/plugins/") ? "plugin" : "core", plugin_category: path.startsWith("kbide/plugins/") ? path.split("/")[2] : null, ids, count: ids.length, ids_with_original_generator: ids.filter((id) => blocks.find((block) => block.id === id).archive.generators.length), ids_with_kb_exact_mentions: ids.filter((id) => blocks.find((block) => block.id === id).kb_exact_id_mentions.matches.length), ids_with_portable_components: ids.filter((id) => componentMap.has(id)), ids_with_recipe_subsets: ids.filter((id) => recipeMap.has(id)) };
});
const functionInventory = exampleFiles.map((path) => {
  const text = textByFile.get(path);
  const pattern = /^[ \t]*(?:(?:static|inline|const|unsigned|signed|struct)\s+)*(?:[A-Za-z_]\w*[ \t*]+)+([A-Za-z_]\w*)\s*\([^;{}]*\)\s*\{/gm;
  const definitions = [...text.matchAll(pattern)].filter((match) => !["if", "while", "for", "switch"].includes(match[1])).map((match) => ({ name: match[1], line: text.slice(0, match.index).split("\n").length }));
  const currentHash = sha(readFileSync(join(root, path)));
  const matches = (sources) => sources.some((source) => source.path === path && source.sha256 === currentHash) && sources.every((source) => existsSync(join(root, source.path)) && sha(readFileSync(join(root, source.path))) === source.sha256);
  const hostSuites = expansion.host_c_checks.filter((suite) => suite.status === "passed" && matches(suite.sources));
  const firmwareBuilds = expansion.firmware_builds.filter((build) => build.exit_code === 0 && matches(build.sources));
  return { path, sha256: currentHash, extraction: "static-C-like-signature-candidates-not-a-C-parser", function_candidates: definitions, arduino_markers: /#include\s*[<"](?:Arduino|Wire|Servo|ESP32Servo)\.h[>"]|\b(?:digitalWrite|analogRead|pinMode)\s*\(/.test(text), full_file_build_status: hostSuites.length || firmwareBuilds.length ? "recorded-source-and-dependency-hash-matched" : "no-current-hash-matched-build-evidence", build_evidence: { path: expansionPath, host_suites: hostSuites.map(({suite, checks}) => ({suite, checks})), firmware_builds: firmwareBuilds.map(({example, board_profile}) => ({example, board_profile})) }, note: "Reference file presence is not a certified implementation of every related block or every board" };
});
const categorySummary = sorted(blocks.map((block) => block.category)).map((name) => ({ name, ids: blocks.filter((block) => block.category === name).map((block) => block.id), archive_definitions: count((block) => block.category === name && block.archive.definitions.length > 0), web_definitions: count((block) => block.category === name && block.web.definitions.length > 0), kb_exact_candidate_ids: count((block) => block.category === name && block.kb_exact_id_mentions.matches.length > 0), host_component_ids: count((block) => block.category === name && block.portable_component_ids.length > 0), recipe_subset_ids: count((block) => block.category === name && block.firmware_recipe_subsets.length > 0) }));
const scopedIds = blocks.filter((block) => block.portable_component_ids.length || block.firmware_recipe_subsets.length).map((block) => block.id);
const summary = {
  archive: summaryByScope("archive"), web: summaryByScope("web"),
  raw_archive_core_definition_assignments: rawAssignments.filter((item) => item.scope === "archive" && item.kind === "block" && !item.path.startsWith("kbide/plugins/")).length,
  raw_archive_plugin_definition_assignments: rawAssignments.filter((item) => item.scope === "archive" && item.kind === "block" && item.path.startsWith("kbide/plugins/")).length,
  archive_plugin_modules: modules.filter((module) => module.origin === "plugin").length,
  archive_plugin_categories: sorted(modules.filter((module) => module.origin === "plugin").map((module) => module.plugin_category)),
  toolbox_entries: evidence.toolbox.reduce((total, item) => total + item.types.length, 0),
  toolbox_unique_ids: new Set(evidence.toolbox.flatMap((item) => item.types)).size,
  toolbox_dynamic_categories: evidence.toolbox.filter((item) => item.custom),
  sample_unique_ids: evidence.sample_block_types.all_types.length,
  sample_active_unique_ids: evidence.sample_block_types.active_types.length,
  union_inventory_ids: blocks.length,
  editor_mutator_ids: blocks.filter((block) => block.record_kind === "editor-mutator").map((block) => block.id),
  generator_helper_ids: blocks.filter((block) => block.record_kind === "generator-helper").map((block) => block.id),
  exact_kb_candidate_ids: count((block) => block.kb_exact_id_mentions.matches.length > 0),
  exact_kb_unambiguous_candidate_ids: count((block) => block.kb_exact_id_mentions.matches.length > 0 && !block.kb_exact_id_mentions.generic_word_ambiguous),
  portable_component_mapped_ids: count((block) => block.portable_component_ids.length > 0),
  fixed_recipe_subset_mapped_ids: count((block) => block.firmware_recipe_subsets.length > 0),
  ids_with_any_evidenced_subset: scopedIds,
  fully_verified_block_adapters: 0,
  physically_verified_block_equivalents: 0,
};
const result = {
  schema_version: 1, audit_date: "2026-10-17", timezone: "Asia/Bangkok",
  conclusion: "Incomplete for every-board/every-block compatibility; inventory is exhaustive only for the captured sources and observed static IDs, not every KidBright version or third-party extension",
  method: "Reconcile committed static research evidence with hash-verified no-execution AST index; compute static IDs, sources and lexical KB candidates; manually map only inspected scoped C helpers and two recipe subsets to recorded software evidence. No live AI, browser, network service or physical board execution in this audit.",
  inputs: [evidencePath, indexPath, pilotPath, expansionPath].map((path) => ({ path, sha256: sha(readFileSync(join(root, path))) })),
  repository_snapshot: supportFiles.map((path) => ({ path, sha256: sha(readFileSync(join(root, path))) })),
  caveats: [
    "Generator presence in a reference archive does not mean that C implementation exists in vibeKidbright, or that the plugin is loaded in a given board/session.",
    "Simulator JavaScript and desktop DEV_IO/DEV_I2C C++ output do not certify ESP-IDF C or real device behavior.",
    "Generic functions, math, loops, strings and lists can be expressed in C, but arbitrary AI generation is not an audited block adapter or equivalence proof.",
    "An exact block-ID mention in a rule is a lexical candidate only; absence can also miss equivalent human-readable wording.",
    "AST handles direct static assignments and literal JSON arrays only; computed factories, runtime registration and third-party versions can introduce additional IDs.",
    "The original generator smoke checks cover eight cases, not every input, composition, hardware configuration or SDK.",
    "No explicit storage/NVS/filesystem block was identified in this snapshot; ESP-IDF/library capability does not count as a documented block contract.",
    "The historical pilot evidence covers two recipe builds; expansion evidence separately covers seven permitted profile/recipe pairs. Neither certifies physical peripherals or complete original blocks.",
  ],
  summary, categories: categorySummary, archive_modules: modules, portable_components: Object.values(components),
  pilot_validation: { path: pilotPath, sdk: pilot.sdk, physical_board_tested: pilot.physical_board_tested, live_ai_provider_tested: pilot.live_ai_provider_tested, original_generator_differential_tested: pilot.original_generator_differential_tested },
  expansion_validation: { path: expansionPath, board_catalog: expansion.board_catalog, firmware_builds: expansion.firmware_builds.length, host_checks: expansion.host_c_checks.reduce((sum, suite) => sum + suite.checks, 0), scope: "Additional numeric matrix/music primitives and recipes; no new whole-block equivalence mapping inferred" },
  reference_function_inventory: functionInventory,
  blocks,
};

const cell = (value) => String(value).replaceAll("|", "\\|").replaceAll("\n", " ");
const table = (headers, rows) => [headers, headers.map(() => "---"), ...rows].map((row) => `| ${row.map(cell).join(" | ")} |`).join("\n");
const refs = (items) => items.map(sourceRef).join("<br>") || "—";
const marker = (items) => items.length ? "มี" : "—";
const lines = [
  "# ตรวจความครอบคลุมบล็อกและฟังก์ชัน KidBright", "", "ปรับข้อมูล: **17 ตุลาคม 2026 — Asia/Bangkok**", "",
  "**ยังไม่ครบทุกบอร์ด ทุกฟังก์ชัน และทุกบล็อก** การจับคู่รายบล็อกในตารางนี้ยังจำกัดแกน C และสอง recipe เดิม ส่วนรอบใหม่มีตัวอย่างรวม 4 recipe และ build 7 คู่ profile/recipe ตาม [หลักฐาน expansion](research/kidbright-expansion-2026-10-17-validation.json) primitive จอ/เพลงใหม่ยังไม่ถูกนับเป็น block adapter ที่เทียบเท่าต้นฉบับ การมีคำใน `.md` หรือคอมไพล์ผ่านไม่แปลว่ารองรับทั้งระบบ", "",
  "รายงานสร้างโดย `node scripts/audit-kidbright-coverage.mjs` จากหลักฐานที่เก็บใน repository จึงทำซ้ำได้โดยไม่ต้องใช้ ZIP, TEMP หรืออินเทอร์เน็ต สคริปต์ไม่ execute โค้ดอ้างอิง หากต้องการตรวจ hash ซอร์ส cache เพิ่ม ใช้ `--verify-source <research-cache>`; การเก็บ AST ใหม่ใช้ `--capture-source <research-cache>` และต้องมี TypeScript ตาม dev dependencies", "",
  "หลักฐาน: [inventory JSON](research/kidbright-coverage-2026-10-16.json), [AST source index](research/kidbright-static-source-index-2026-10-16.json), [ต้นแบบเดิม](research/kidbright-2026-10-15-evidence.json), [ผลทดสอบ pilot](research/kidbright-pilot-2026-10-16-validation.json). `K:path:line` อ้างจาก application root ภายใน ZIP และ `W:path:line` อ้างจากซอร์สเว็บ snapshot 15 ตุลาคม 2026 ไม่ใช่สถานะเว็บสด", "",
  "## จำนวนที่ตรวจพบ", "",
  table(["รายการ", "จำนวน/ขอบเขต"], [
    ["Archive: นิยามแบบ custom ที่พบใน catalog เดิม", `${summary.archive.raw_custom_unique_definition_ids} IDs (${summary.raw_archive_core_definition_assignments} core + ${summary.raw_archive_plugin_definition_assignments} plugin assignments)`],
    ["Archive: custom IDs ที่เป็น AST assignment จริง", summary.archive.active_custom_unique_definition_ids],
    ["Archive: built-in Blockly IDs เพิ่มเติมในไฟล์ compressed", summary.archive.active_builtin_unique_definition_ids],
    ["Archive: union ของนิยามที่เป็น AST จริง", summary.archive.total_active_unique_definition_ids],
    ["Archive: bundled vertical editor asset IDs ที่ไม่รวมในยอด KidBright หลัก", summary.archive.active_bundled_vertical_asset_unique_ids],
    ["Web: นิยาม custom ใน catalog เดิม / AST จริง", `${summary.web.raw_custom_unique_definition_ids} / ${summary.web.active_custom_unique_definition_ids}`],
    ["Web: built-in Blockly IDs / union นิยาม AST", `${summary.web.active_builtin_unique_definition_ids} / ${summary.web.total_active_unique_definition_ids}`],
    ["Plugin modules / หมวด plugin ใน Archive", `${summary.archive_plugin_modules} / ${summary.archive_plugin_categories.length}`],
    ["Web toolbox: entries / unique IDs", `${summary.toolbox_entries} / ${summary.toolbox_unique_ids}`],
    ["ตัวอย่างเดิม: IDs รวม shadow / active", `${summary.sample_unique_ids} / ${summary.sample_active_unique_ids}`],
    ["Inventory รวม definition, generator, toolbox, sample และ infrastructure", summary.union_inventory_ids],
    ["IDs ที่พบตัวสะกดตรงใน KB/system prompt (candidate เท่านั้น)", summary.exact_kb_candidate_ids],
    ["IDs ที่เป็น exact candidate และไม่ใช่คำทั่วไป function/text", summary.exact_kb_unambiguous_candidate_ids],
    ["IDs ที่โยงกับ host C component / fixed recipe subset", `${summary.portable_component_mapped_ids} / ${summary.fixed_recipe_subset_mapped_ids}`],
    ["IDs ที่มี subset อย่างน้อยหนึ่งส่วน (ไม่บวกซ้ำ)", scopedIds.length],
    ["Full block adapters ที่พิสูจน์ครบ / physical equivalence", "0 / 0"],
  ]), "",
  "ตัวเลข built-in รวม mutator ของ editor ที่ไม่ใช่คำสั่ง runtime และอาจซ้ำชื่อกับ custom จึงใช้ union เมื่อรวมจำนวน ส่วน generator helpers แสดงเพื่อปิดช่องว่าง inventory แต่ไม่ควรถูกนับเป็นฟังก์ชันฮาร์ดแวร์ Dynamic Variables toolbox ไม่มี static IDs ใน XML จึงต้องตรวจ definitions/generators เพิ่มด้วย", "",
  `ไฟล์ \`blocks_compressed_vertical.js\` ที่อยู่ใน ZIP มีบล็อก Scratch เช่น motion/looks/sound/data เพิ่มอีก ${summary.archive.active_bundled_vertical_asset_unique_ids} IDs; ยังไม่ได้พิสูจน์ว่า KidBright โหลดใช้ จึงเก็บใน appendix/inventory เป็น bundled assets แยกจากนิยามหลัก ${summary.archive.active_custom_unique_definition_ids} IDs และ built-in Blockly ${summary.archive.active_builtin_unique_definition_ids} IDs ไม่ใช้การที่ไฟล์ถูกบรรจุเป็นหลักฐานว่ารองรับ ESP-IDF`, "",
  `Archive มีชื่อซ้ำระหว่าง custom/built-in ${summary.archive.custom_builtin_overlapping_ids.length} IDs จึงรวมได้ ${summary.archive.total_active_unique_definition_ids} ไม่ใช่ผลบวกตรง ๆ ส่วน Web ซ้ำ ${summary.web.custom_builtin_overlapping_ids.length} IDs และ union ${summary.web.total_active_unique_definition_ids} รายชื่อที่ซ้ำทั้งหมดอยู่ใน JSON`, "",
  "คำทั่วไป `function` และ `text` ปรากฏในกฎหลายบริบทและอาจไม่หมายถึง block ID จริง จึงแยกเป็น ambiguous lexical candidates; ทั้งตัวเลข exact และการไม่พบชื่อยังไม่ตัดสิน semantic coverage ของคำอธิบายภาษามนุษย์", "",
  `AST ตัด comments ออกและตรวจ aliases เพิ่ม เช่น \`procedures_defnoreturn = procedures_defreturn\` ผลเทียบตำแหน่ง catalog เดิมพบ generator mentions ที่ไม่ใช่ active assignment ${summary.archive.raw_generator_mentions_without_active_assignment.length} ตำแหน่งใน Archive และ ${summary.web.raw_generator_mentions_without_active_assignment.length} ตำแหน่งใน Web จึงไม่พบการนับ comment เกินใน catalog เดิม แต่พบ builtin aliases เพิ่ม ${summary.archive.builtin_generator_aliases.length} รายการต่อฝั่ง รายละเอียดอยู่ใน \`summary.*.raw_generator_mentions_without_active_assignment\` และ \`builtin_generator_aliases\``, "",
  "## หลักฐานแต่ละสถานะหมายถึงอะไร", "",
  "- **นิยาม/generator:** พบ declaration จริงนอก comment ใน source AST; ยังไม่ยืนยันว่า plugin โหลดหรือรันสำเร็จ", 
  "- **KB candidate:** พบชื่อ ID ตรงใน Markdown/system prompt; ยังไม่ยืนยันความครบถ้วนของ API, พิน,หน่วย, lifecycle หรือค่าผิดพลาด", 
  "- **Host component:** มี C helper และ host tests ของ subset ที่ระบุ; ไม่มีตัวแปลงบล็อกทั่วไปหรือความเข้ากันได้ทุกค่าพารามิเตอร์", 
  "- **Recipe subset:** มีงานสำเร็จรูปขอบเขตตายตัวและ hash โค้ดตรงผล build ที่บันทึก; ไม่ใช่ทุกตัวเลือกของบล็อกนั้น", 
  "- **—:** ไม่พบหลักฐานชั้นนั้นใน snapshot นี้; ไม่ควรแปลว่า AI เขียน C ในเรื่องนั้นไม่ได้เลย", "",
  "การจัดกลุ่มตามชื่อบล็อก/path เป็นเครื่องมือช่วยค้น ไม่แทนการอ่าน semantics และทดสอบ โค้ด C++ ของ plugin เดิมไม่ถูกนับเป็น implementation ของแอป และ JavaScript simulator ไม่ถูกนับเป็น firmware", "",
  "## ความครอบคลุมรายหมวด", "",
  table(["หมวด", "IDs", "Archive defs", "Web defs", "KB candidate", "Host subset", "Recipe subset"], categorySummary.map((item) => [item.name, item.ids.length, item.archive_definitions, item.web_definitions, item.kb_exact_candidate_ids, item.host_component_ids, item.recipe_subset_ids])), "",
  "## C ที่มีหลักฐานตรวจจริงและข้อจำกัด", "",
  table(["Component", "API", "ขอบเขตที่ตรวจ / ข้อยกเว้น"], Object.values(components).map((item) => [item.id, item.symbols.map((symbol) => `\`${symbol}\``).join(", "), item.scope])), "",
  "Host tests อ้างจาก `src-tauri/tests/kidbright_core_test.c` และเป็น test ของ implementation ปัจจุบัน มิใช่การรันเทียบทุก input กับ runtime เดิม PID/SMA มี behavior repairs ตาม `knowledge_base/compatibility/README.md` จึงไม่อ้าง bit-exact ทุกกรณี มีเพียง codec ASCII ของ ESP-NOW; การเชื่อมต่อวิทยุจริงยังไม่มีใน pilot", "",
  "Recipe `button_buzzer` ใช้ S1 GPIO16 active-low และ buzzer GPIO13 เฉพาะ A4 440 Hz 500 ms; `temperature_serial` อ่าน LM73 SDA4/SCL5 ที่ 0x4D ทุก 500 ms ทั้งสองใช้ target esp32 และผล build ที่บันทึกเป็น ESP-IDF 5.5.4 โดยสร้างคนละ profile (`kidbright32_v13` และ `kidbright32_rev31`) ยังไม่ครอบคลุมทุกคู่ recipe/profile หรือทดลองบอร์ดจริง", "",
  "## ช่องว่างที่ต้องปิดก่อนกล่าวว่าครบ", "",
  "1. สัญญารายบล็อก: inputs/defaults/types/units/ranges/return/error/side effects และ lifecycle พร้อม reference source ต้องมีครบ ไม่ใช้เพียงรายการชื่อ", 
  "2. บล็อกพื้นฐาน: variables, functions, logic, loops, lists และ text ต้องตรวจการกำหนดขอบเขต ความยาวหน่วยความจำ การแปลงชนิด การเรียกซ้ำ และ composition แม้ C เขียนสิ่งเหล่านี้ได้", 
  "3. FreeRTOS/tasks: ต้องพิสูจน์ start/stop, scheduling, delay quantization, shared state, callbacks และ race จากหลายบล็อก; `for (;;)` ในตัวอย่างเดียวไม่แทน `advance_task`", 
  "4. Display/audio/clock: matrix mapping/font/scroll/ready, OLED coordinates/font/UTF-8, LCD/SPI, LED strips, tempo/scale/volume และ RTC/calendar ต้องมี API และผลตรวจของตัวเอง", 
  "5. I/O/sensors/control: ADC calibration, sensor revisions, MPU/encoder, motor direction/PWM/stop, servo pulse limits, iKB expansion, EMA และ calibration defaults ยังไม่มีหลักฐานทดสอบครบ", 
  "6. Network/cloud: ESP-NOW protocol/channel/peers/callback ownership, BLE/Bluetooth, MQTT/NETPIE/Firebase/IoT, LINE/IFTTT ต้องตรวจ API และบริการปัจจุบันก่อนใช้; snapshot ไม่รับรองว่าบริการยังใช้งานได้", 
  "7. Storage: ไม่พบบล็อก local storage/NVS/filesystem ที่ชัดเจนใน inventory นี้ จึงยังไม่มีสัญญาบล็อกสำหรับ persist/recover ไม่ใช้ความสามารถของ ESP-IDF มาทดแทนหลักฐาน", 
  "8. บอร์ด/SDK: ต้องตรวจทุก board revision พร้อม pin/bus/address/resource conflicts และ build matrix จริง รวมทั้งบอร์ดที่ไม่มี LM73 หรือ GPIO ต่างกัน", 
  "9. ทุกภาษา: ชุดนี้ไม่ได้ทดสอบความสามารถภาษาโมเดล การใช้ภาษาตามข้อความล่าสุดเป็น policy คนละชั้นกับชื่อ API/ID และ font rendering บนฮาร์ดแวร์ ไม่อาจรับรองทุกภาษาเพียงเพราะ prompt ระบุไว้", 
  "10. End-to-end: ต้องทดสอบคำขอภาษาธรรมชาติ → เลือกบอร์ด/สร้าง → Keep/Undo → build → behavior บอร์ดจริง และเปรียบเทียบกับต้นแบบหลายตัวเลือก โดยไม่ใช้ผล compile แทนผลฮาร์ดแวร์", "",
  "## โมดูล Archive ทั้งหมดที่พบ static block definitions", "",
  table(["Source module", "ประเภท", "IDs", "มี original generator", "KB candidate", "Host subset", "Recipe subset"], modules.map((item) => [`\`${item.path}\``, item.origin, item.count, item.ids_with_original_generator.length, item.ids_with_kb_exact_mentions.length, item.ids_with_portable_components.length, item.ids_with_recipe_subsets.length])), "",
  "## Definitions ที่ไม่พบ generator ใน snapshot", "",
  "รายการนี้รวม editor mutators ซึ่งไม่จำเป็นต้องมี runtime generator และอาจรวมบล็อกที่มีตัวกำหนดแบบ dynamic นอกขอบเขต static parser จึงเป็นรายการตรวจต่อ ไม่ใช่ข้อพิสูจน์ว่าต้นแบบเสียทุกตัว", "",
  ...["archive", "web"].flatMap((scope) => [`**${scope}:** ${summary[scope].definitions_without_observed_generator.map((id) => `\`${id}\``).join(", ") || "ไม่มี"}`, ""]),
  "## บัญชีทุก ID และสถานะ", "",
  "ตารางแยกตามหมวด รวมทุก ID ใน inventory JSON คอลัมน์ `K def/gen` และ `W def/gen` นับ source declarations ที่พบ (ไม่ใช่จำนวนบอร์ดที่รองรับ) แหล่งอ้างอิงสั้นแสดง definition แรกหรือ generator แรก; JSON มีแหล่งอ้างอิงทั้งหมดพร้อมบรรทัด รวมถึงตำแหน่ง candidate ในกฎ", "",
];
for (const item of categorySummary) {
  lines.push(`### ${item.name}`, "", table(["Block ID", "K def/gen", "W def/gen", "Toolbox", "Sample", "KB candidate", "Host subset", "Recipe subset", "แหล่งอ้างอิงแรก"], blocks.filter((block) => block.category === item.name).map((block) => [
    `\`${block.id}\``, `${block.archive.definitions.length}/${block.archive.generators.length}`, `${block.web.definitions.length}/${block.web.generators.length}`,
    marker(block.toolbox_categories), marker(block.sample_files), block.kb_exact_id_mentions.matches.length ? `${block.kb_exact_id_mentions.matches.length} จุด` : "—",
    block.portable_component_ids.join(", ") || "—", block.firmware_recipe_subsets.map((mapping) => mapping.recipe).join(", ") || "—",
    refs([...(block.archive.definitions.length ? [block.archive.definitions[0]] : []), ...(!block.archive.definitions.length && block.web.definitions.length ? [block.web.definitions[0]] : []), ...(!block.archive.definitions.length && !block.web.definitions.length ? [...block.archive.generators, ...block.web.generators].slice(0, 1) : [])]),
  ])), "");
}
lines.push("## Inventory ฟังก์ชันในไฟล์ตัวอย่างปัจจุบัน", "", "รายการต่อไปเป็น static signature candidates เพื่อช่วยตรวจว่าฟังก์ชันอยู่ที่ไฟล์ใด ไม่ใช่ผล compile หรือการรับรองว่าทุกไฟล์ใช้ได้พร้อมกัน ทั้งยังไม่รวมโค้ดใน fenced Markdown เป็น library ที่พร้อม link", "", table(["ไฟล์", "Function candidates", "Arduino markers", "หลักฐาน build"], functionInventory.map((item) => [`\`${item.path}\``, item.function_candidates.map((fn) => `\`${fn.name}\`:${fn.line}`).join(", ") || "—", item.arduino_markers ? "พบ" : "ไม่พบ", item.full_file_build_status])), "", "## ทำซ้ำและตรวจความสดของรายงาน", "", "```powershell", "node scripts/audit-kidbright-coverage.mjs", "node scripts/audit-kidbright-coverage.mjs --check", "# Optional: compare the original captured files, without running them", "node scripts/audit-kidbright-coverage.mjs --verify-source <research-cache>", "```", "", "เมื่อ `.md`, system prompt, C helper หรือหลักฐานเปลี่ยน ให้สร้างรายงานใหม่ `--check` ตรวจว่าผลลัพธ์ที่เก็บยังตรงกับไฟล์ปัจจุบัน ไม่ได้ทดสอบภาษา AI หรือฮาร์ดแวร์ ผล inventory นี้มีขอบเขตเฉพาะ source snapshot ที่บันทึกและ static registrations ที่ตรวจพบ จึงไม่ครอบคลุม KidBright ทุกเวอร์ชันหรือ plugins ที่ไม่ได้อยู่ในต้นแบบ", "");
const artifacts = [[outputPath, `${JSON.stringify(result, null, 2)}\n`], [reportPath, lines.join("\n")]];
if (args.includes("--check")) {
  for (const [path, contents] of artifacts) {
    if (!existsSync(join(root, path)) || read(path) !== contents) throw new Error(`Stale coverage artifact: ${path}; run node scripts/audit-kidbright-coverage.mjs`);
  }
  console.log(`Coverage artifacts current: ${blocks.length} inventory IDs; ${scopedIds.length} have scoped evidence; zero full-block or physical certification`);
} else {
  for (const [path, contents] of artifacts) writeFileSync(join(root, path), contents);
  console.log(`Wrote ${outputPath} and ${reportPath}`);
  console.log(JSON.stringify({ archive_custom_ids: summary.archive.active_custom_unique_definition_ids, archive_builtin_ids: summary.archive.active_builtin_unique_definition_ids, archive_union_ids: summary.archive.total_active_unique_definition_ids, web_custom_ids: summary.web.active_custom_unique_definition_ids, web_union_ids: summary.web.total_active_unique_definition_ids, bundled_vertical_ids: summary.archive.active_bundled_vertical_asset_unique_ids, inventory_ids: blocks.length, scoped_ids: scopedIds.length }, null, 2));
}
