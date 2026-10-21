# iP matrix A–Z user evidence — 2026-10-19

The user supplied a complete C program and explicitly reported that it works on
their KidBright32iP. The source is preserved without code edits at
`knowledge_base/sensor_examples/kidbright32ip_matrix_az_user_passed.c` and in the
bundled knowledge copy. It contains SDA21/SCL22, I2C_NUM_0, 0x70 at 100 kHz,
oscillator/display/brightness commands, interleaved 16-byte display RAM,
column-major A–Z font with reverse_bits, and an 80 ms right-to-left scroll.
Its I2C implementation uses the legacy driver family consistently.

## Evidence scope

- Physical display outcome: **USER_REPORTED_PASS**, not an agent-executed test.
- User PCB revision, installed SDK version, build logs and independent serial or
  video capture: **NOT_PROVIDED**.
- Agent firmware compilation and physical execution of this supplied program:
  **NOT_EXECUTED**.
- Real-provider replay of the creation workflow: **NOT_EXECUTED**.
- Complete iP peripheral support and iA/iP equivalence: **NOT_ESTABLISHED**.

The reported block was in the app's rules/reference coverage: revision and
unrelated sensor questions prevented use of task-specific evidence. The previous
increment corrected those rules. This increment supplies the concrete working
reference and supersedes earlier “no iP physical result reported” wording only
for this user's display task. The support contract, model table, iP document,
catalog note and system prompt now direct the AI to that source. No executable
board preset or Block ID is added. Keep/Undo, privacy gates, SDK configuration,
production ranking, model and frozen evaluation set are preserved.

The display buffer and bit reversal agree structurally with the existing iA text
reference. This does not establish that the two boards are identical: the
provided product evidence distinguishes iP phototransistor from iA LDR and the
specified iA acceleration sensor. That sensor is not used by this program.

Exact source hashes, commands and current checks are recorded in
`ip-matrix-verification-20261019.json` and `ip-matrix-changes-20261019.json`.
Historical NSIS 3.7.3 remains unchanged; a no-bundle local build is development
output, not a newly qualified installer or a hardware/AI quality PASS.
