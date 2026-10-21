import { readFileSync } from "node:fs";
import { strict as assert } from "node:assert";
import { createHash } from "node:crypto";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const catalog = JSON.parse(readFileSync(new URL("../knowledge_base/compatibility/board_catalog.json", import.meta.url), "utf8"));
assert.equal(catalog.schema_version, 1);
assert.equal(catalog.as_of, "2026-10-17");
assert.ok(Array.isArray(catalog.boards) && catalog.boards.length > 0);
assert.ok(Array.isArray(catalog.sources) && catalog.sources.length > 0);
assert.ok(catalog.deferred_families.includes("kidbright_uai"));
assert.ok(catalog.deferred_families.includes("kidbright_uai_plus"));
const sourceIds = new Set(catalog.sources.map(source => source.id));
assert.equal(sourceIds.size, catalog.sources.length, "Duplicate source identifiers");
for (const source of catalog.sources) {
  assert.ok(source.title && source.issuer && source.evidence_kind);
  assert.ok(source.url || source.path, `Source ${source.id} needs a URL or local path`);
  if (source.url) assert.match(source.url, /^https:\/\//);
  for (const [path, digest] of [[source.path, source.sha256], [source.local_copy, source.local_copy_sha256]]) {
    if (path && digest) assert.equal(createHash("sha256").update(readFileSync(resolve(root, path))).digest("hex"), digest, `Changed source: ${path}`);
  }
}
const ids = new Set();
let executable = 0;
const executableIds = new Set();
for (const board of catalog.boards) {
  assert.ok(board.id && board.label && !ids.has(board.id), `Duplicate/empty board ${board.id}`);
  ids.add(board.id);
  assert.ok(!catalog.deferred_families.includes(board.id), "Deferred AI boards cannot become active catalog profiles");
  assert.ok(["pilot", "needs_validation", "historical_unverified"].includes(board.status));
  assert.ok([null, "esp32", "esp8266"].includes(board.target));
  for (const key of ["aliases", "source_ids", "pins", "peripherals", "unknowns", "conflicts", "notes"]) assert.ok(Array.isArray(board[key]), `${board.id}.${key}`);
  assert.ok(board.unknowns.length, `${board.id} must state remaining verification limits`);
  const references = [...board.source_ids, ...board.pins.flatMap(pin => pin.source_ids), ...board.peripherals.flatMap(device => device.source_ids)];
  for (const reference of references) assert.ok(sourceIds.has(reference), `Unknown source ${reference} on ${board.id}`);
  for (const pin of board.pins) {
    assert.ok(pin.signal && pin.source_ids.length, `Unsourced signal on ${board.id}`);
    assert.ok(Number.isInteger(pin.gpio) && pin.gpio >= 0 && pin.gpio <= (board.target === "esp8266" ? 16 : 39));
  }
  if (board.reference_profile_id !== null) {
    assert.ok(["kidbright32_v13", "kidbright32_rev31"].includes(board.reference_profile_id));
    assert.equal(board.status, "pilot");
    assert.equal(board.target, "esp32");
    executable++;
    executableIds.add(board.reference_profile_id);
  }
}
assert.equal(executable, 2, "Catalog comparison must not silently add executable profiles");
assert.deepEqual([...executableIds].sort(), ["kidbright32_rev31", "kidbright32_v13"]);
for (const unresolved of catalog.unresolved_labels ?? []) {
  assert.ok(unresolved.label && unresolved.reason && unresolved.candidate_board_ids.length);
  for (const candidate of unresolved.candidate_board_ids) assert.ok(ids.has(candidate), `Unknown alias candidate ${candidate}`);
}
console.log(`KidBright catalog checked: ${catalog.boards.length} compared entries, ${executable} scoped profiles; muAI and muAI Plus deferred.`);
