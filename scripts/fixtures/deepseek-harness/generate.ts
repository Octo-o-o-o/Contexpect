// SPDX-License-Identifier: Apache-2.0
//
// Synthetic DeepSeek Harness (DSH) session fixture generator.
//
// Runs INSIDE a read-only DSH checkout so that the fixture and its expected
// values come from DSH's own public API, never from ContextView's importer:
//
//   pnpm --dir <dsh-checkout> exec tsx <ContextView>/scripts/fixtures/deepseek-harness/generate.ts \
//       --out <ContextView>/acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1
//
// Dependencies: the MIT-licensed `@deepseek-ai/dsh-session` (event log,
// surface fold, request-header fold, interrupted-turn closers) and the
// header/row encoders of `@deepseek-ai/dsh-session-persistence-jsonl`, both
// imported from the checkout the script runs in (its cwd). This file itself
// is Apache-2.0; it writes nothing into the checkout.
//
// Output (all synthetic — no real paths, tokens or user names):
//   session.jsonl   the log as the JSONL backend writes it (plain, unpacked)
//   expected.json   per request prefix: canonical header digest, derived
//                   message digests, surface nodes and replacements; the
//                   interrupted-tail closers
//   meta.json       dsh_sha, dsh_session_version, node_version,
//                   script_sha256, generated_at, live_tested: false, license

import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const FIXTURE_SCHEMA = 'ctxpect-dsh-fixture-expected-v1'
const META_SCHEMA = 'ctxpect-dsh-fixture-meta-v1'
const SESSION_ID = 'ctxpect-synthetic-0001'
const CWD = '/tmp/ctxpect-fixture/project'
const T0 = 1_700_000_000_000

function arg(name: string): string {
  const index = process.argv.indexOf(name)
  if (index === -1 || index + 1 >= process.argv.length) {
    throw new Error(`missing ${name} <value>`)
  }
  return process.argv[index + 1] as string
}

/** Sorted-key canonical JSON, matching ctxpect_schema::canonical_json for the ASCII, integer-only values used here. */
function canonicalize(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(canonicalize).join(',')}]`
  const keys = Object.keys(value as Record<string, unknown>).sort()
  return `{${keys.map(key => `${JSON.stringify(key)}:${canonicalize((value as Record<string, unknown>)[key])}`).join(',')}}`
}

function sha256(text: string): string {
  return createHash('sha256').update(text, 'utf8').digest('hex')
}

async function main(): Promise<void> {
  const out = resolve(arg('--out'))
  const dsh = process.cwd()
  // Public API of the pinned checkout. `./src/*` is an exported subpath of
  // both packages, so these are public entry points, not internals.
  const sessionModule = await import(pathToFileURL(join(dsh, 'packages/core/session/src/index.ts')).href)
  const formatModule = await import(pathToFileURL(join(dsh, 'packages/session/session-persistence-jsonl/src/format.ts')).href)
  const {
    Session, SessionId, SESSION_FORMAT_VERSION, KNOWN_SESSION_EVENT_TYPES,
    foldSurface, foldRequestHeader, interruptedTurnClosers, TOOL_OUTCOME_UNKNOWN, TOOL_NOT_STARTED,
  } = sessionModule
  const { toHeaderLine, eventLines } = formatModule

  // A non-integer in the header config: the importer must keep the lexeme
  // JSON.stringify wrote so `header_digest` matches DSH's own hash.
  const model = { provider: 'deepseek', model: 'deepseek-chat', config: { temperature: 0.5, top_p: 0.95, maxTokens: 4096 } }
  const tools = [{ name: 'bash', description: 'Run a shell command', parameters: { type: 'object', properties: { cmd: { type: 'string' } }, required: ['cmd'] } }]
  const msg = (n: number) => `m-${String(n).padStart(4, '0')}`
  let seq = 0
  const events: Record<string, unknown>[] = []
  const push = (event: Record<string, unknown>) => {
    events.push({ ...event, seq, time: T0 + seq * 1000 })
    seq += 1
  }

  // Turn 1: a human prompt, injected instructions context, one model call
  // with a tool round-trip, a second call after a header change.
  push({ type: 'turn/start', data: { turn: 1 } })
  push({ type: 'user/message', data: { id: msg(1), role: 'user', content: [{ type: 'text', text: 'List the files in the project.' }], source: { kind: 'user' } }, surfaceOp: 'append' })
  push({ type: 'user/message', data: { id: msg(2), role: 'user', content: [{ type: 'text', text: '<system-reminder>AGENTS.md: keep answers brief.</system-reminder>' }], source: { kind: 'plugin', plugin: 'agent-instructions', form: 'instructions' } }, surfaceOp: 'append' })
  push({ type: 'step/start', data: { turn: 1, step: 1 } })
  push({ type: 'request/context', data: { ...model, contextWindow: 128000 } })
  push({ type: 'request/header', data: { header: { config: model, system: 'You are a coding agent.', tools }, reason: 'initial' } })
  push({ type: 'assistant/chunk', data: { turn: 1, step: 1, chunk: { type: 'block-start', index: 0, blockType: 'text' } } })
  push({ type: 'assistant/chunk', data: { turn: 1, step: 1, chunk: { type: 'text-delta', index: 0, text: 'Let me ' } } })
  push({ type: 'assistant/chunk', data: { turn: 1, step: 1, chunk: { type: 'text-delta', index: 0, text: 'check.' } } })
  push({ type: 'assistant/message', data: {
    turn: 1, step: 1,
    message: { id: msg(3), role: 'assistant', content: [{ type: 'text', text: 'Let me check.' }, { type: 'tool-call', id: 'call-0001', name: 'bash', arguments: '{"cmd":"ls"}' }], source: { kind: 'model', ...model } },
    usage: { inputTokens: 120, outputTokens: 20, totalTokens: 140 },
  }, surfaceOp: 'append', sourceEventSeqs: [6, 7, 8] })
  push({ type: 'tool/call', data: { turn: 1, step: 1, callId: 'call-0001', name: 'bash', arguments: '{"cmd":"ls"}' } })
  push({ type: 'tool/result', data: {
    turn: 1, step: 1,
    message: { id: msg(4), role: 'user', source: { kind: 'tool', callId: 'call-0001' }, content: [{ type: 'tool-result', toolCallId: 'call-0001', isError: false, content: [{ type: 'text', text: 'AGENTS.md README.md src' }] }] },
  }, surfaceOp: 'append', sourceEventSeqs: [10] })
  push({ type: 'step/end', data: { turn: 1, step: 1 } })
  push({ type: 'step/start', data: { turn: 1, step: 2 } })
  push({ type: 'request/header', data: { header: { config: model, system: 'You are a coding agent. Be brief.', tools }, reason: 'change', startsSeries: true } })
  push({ type: 'assistant/message', data: {
    turn: 1, step: 2,
    message: { id: msg(5), role: 'assistant', content: [{ type: 'text', text: 'Three entries: AGENTS.md, README.md, src.' }], source: { kind: 'model', ...model } },
    usage: { inputTokens: 210, outputTokens: 12, totalTokens: 222, cacheReadTokens: 100 },
  }, surfaceOp: 'append', sourceEventSeqs: [] })
  push({ type: 'step/end', data: { turn: 1, step: 2 } })
  // Step 3 reuses the step-2 header unchanged, so DSH appends no
  // request/header for it (agent.ts:505-518); the request still happens.
  push({ type: 'step/start', data: { turn: 1, step: 3 } })
  push({ type: 'assistant/message', data: {
    turn: 1, step: 3,
    message: { id: msg(9), role: 'assistant', content: [{ type: 'text', text: 'Nothing else to list.' }], source: { kind: 'model', ...model } },
    usage: { inputTokens: 230, outputTokens: 6, totalTokens: 236 },
  }, surfaceOp: 'append', sourceEventSeqs: [] })
  push({ type: 'step/end', data: { turn: 1, step: 3 } })
  push({ type: 'turn/end', data: { turn: 1, reason: { kind: 'completed' } } })

  // A surface replacement (compaction-style): one recall message shadows
  // the surface range 1..11 (nodes 1, 2, 9, 11).
  push({ type: 'user/message', data: { id: msg(6), role: 'user', content: [{ type: 'text', text: 'Summary of the earlier conversation: the user asked for a file listing; the project holds AGENTS.md, README.md and src.' }], source: { kind: 'plugin', plugin: 'compaction', form: 'recall' } }, surfaceOp: { op: 'replace', start: 1, end: 11 }, sourceEventSeqs: [1, 2, 9, 11] })

  // Turn 2: a new prompt, a request after the replacement, a tool call whose
  // result never lands (the log ends mid-step: an interrupted tail).
  push({ type: 'turn/start', data: { turn: 2 } })
  push({ type: 'user/message', data: { id: msg(7), role: 'user', content: [{ type: 'text', text: 'Now show the README.' }], source: { kind: 'user' } }, surfaceOp: 'append' })
  push({ type: 'step/start', data: { turn: 2, step: 1 } })
  push({ type: 'request/header', data: { header: { config: model, system: 'You are a coding agent. Be brief.', tools }, reason: 'series' } })
  push({ type: 'assistant/message', data: {
    turn: 2, step: 1,
    message: { id: msg(8), role: 'assistant', content: [{ type: 'tool-call', id: 'call-0002', name: 'bash', arguments: '{"cmd":"cat README.md"}' }], source: { kind: 'model', ...model } },
    usage: { inputTokens: 90, outputTokens: 15, totalTokens: 105 },
  }, surfaceOp: 'append', sourceEventSeqs: [] })
  push({ type: 'tool/call', data: { turn: 2, step: 1, callId: 'call-0002', name: 'bash', arguments: '{"cmd":"cat README.md"}' } })

  for (const event of events) {
    if (!KNOWN_SESSION_EVENT_TYPES.has(event['type'])) throw new Error(`unknown DSH event type ${String(event['type'])}`)
  }
  const header = { version: SESSION_FORMAT_VERSION, id: SessionId(SESSION_ID), createdAt: T0, cwd: CWD, isSeeded: false, delegationDepth: 0 }
  // DSH validates the whole seed (envelopes, seq contiguity, surface
  // metadata, provenance) through the same path a live append uses.
  const whole = Session.create(SessionId(SESSION_ID), events, header)
  const total = whole.snapshotEvents().length
  if (total < events.length) throw new Error('seed lost events')

  // One request per step (DSH request-reconstruction theorem,
  // packages/core/agent-loop/tests/request-reconstruction.spec.ts): the
  // messages derive at the step/start boundary and the header is the latest
  // request/header snapshot in the prefix up to the first provider output —
  // a header is logged only when it changes (agent.ts:505-518), so a step
  // without one is still a request.
  const requests = []
  for (const event of events) {
    if (event['type'] !== 'step/start') continue
    const startSeq = event['seq'] as number
    const { turn, step } = event['data'] as { turn: number; step: number }
    const dispatch = events.find(e => (e['seq'] as number) > startSeq
      && (e['type'] === 'assistant/chunk' || e['type'] === 'assistant/message')
      && (e['data'] as { turn?: number; step?: number }).turn === turn
      && (e['data'] as { turn?: number; step?: number }).step === step)
    const stepEnd = events.find(e => (e['seq'] as number) > startSeq && e['type'] === 'step/end'
      && (e['data'] as { turn?: number; step?: number }).turn === turn
      && (e['data'] as { turn?: number; step?: number }).step === step)
    const prefixEnd = dispatch ? (dispatch['seq'] as number) : stepEnd ? (stepEnd['seq'] as number) : events.length
    const headerEvent = [...events.slice(0, prefixEnd)].reverse().find(e => e['type'] === 'request/header')
    const atStart = events.slice(0, startSeq + 1)
    const session = Session.create(SessionId(SESSION_ID), atStart, header)
    const surface = foldSurface(atStart)
    const headerFold = foldRequestHeader(events.slice(0, prefixEnd))
    const messages = session.deriveMessages()
    requests.push({
      seq: startSeq,
      turn,
      step,
      header_seq: headerEvent ? headerEvent['seq'] : null,
      header_logged_in_step: headerEvent ? (headerEvent['seq'] as number) > startSeq : false,
      dispatch_seq: dispatch ? dispatch['seq'] : null,
      reason: headerEvent ? (headerEvent['data'] as { reason: string }).reason : null,
      header_digest: headerEvent ? sha256(canonicalize(headerFold)) : null,
      message_count: messages.length,
      messages: messages.map((message: unknown, index: number) => ({
        index,
        role: (message as { role: string }).role,
        digest: sha256(canonicalize(message)),
      })),
      surface_nodes: surface.nodes,
      replacements: surface.replacements.map((r: { seq: number; start: number; end: number; shadowedSeqs: number[] }) => ({
        seq: r.seq, start: r.start, end: r.end, shadowed: r.shadowedSeqs,
      })),
    })
  }
  const closers = interruptedTurnClosers(events)
  const expected = {
    schema: FIXTURE_SCHEMA,
    session_id: SESSION_ID,
    format_version: SESSION_FORMAT_VERSION,
    event_count: events.length,
    requests,
    final_surface: foldSurface(events).nodes,
    tail: {
      interrupted: closers.length > 0,
      closers: closers.map((closer: { type: string; data: Record<string, unknown> }) => ({
        type: closer.type,
        error_code: (closer.data['error'] as { code?: string } | undefined)?.code ?? null,
        call_id: (closer.data['message'] as { source?: { callId?: string } } | undefined)?.source?.callId ?? null,
        reason: (closer.data['reason'] as { kind?: string } | undefined)?.kind ?? null,
      })),
      codes: { TOOL_OUTCOME_UNKNOWN, TOOL_NOT_STARTED },
    },
    request_anchor_rule: 'one request per step/start (request-reconstruction.spec.ts THEOREM): messages derive at the step/start boundary; the header is foldRequestHeader over the prefix up to the first provider output of that step, so a step whose header did not change (agent.ts:505-518 logs request/header only for initial/resume, a header change, or a series start) reuses the latest snapshot.',
    dispatch_evidence_rule: 'agent-loop/src/agent.ts:364-368 — the loop opens the provider stream and appends assistant/chunk per streamed chunk (line 376 / 396: assistant/message assembles them); an assistant/chunk or assistant/message in the step is therefore evidence the request was dispatched. request/header alone (agent.ts:508-517) is appended before dispatch and proves only preparation.',
  }

  mkdirSync(out, { recursive: true })
  const jsonl = `${JSON.stringify(toHeaderLine(header))}\n${eventLines(events, false)}\n`
  writeFileSync(join(out, 'session.jsonl'), jsonl)
  writeFileSync(join(out, 'expected.json'), `${JSON.stringify(expected, null, 2)}\n`)
  const scriptPath = process.argv[1] as string
  const meta = {
    schema: META_SCHEMA,
    dsh_sha: execFileSync('git', ['-C', dsh, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    dsh_session_version: (JSON.parse(readFileSync(join(dsh, 'packages/core/session/package.json'), 'utf8')) as { version: string }).version,
    dsh_license: 'MIT',
    node_version: process.version,
    script: 'scripts/fixtures/deepseek-harness/generate.ts',
    script_sha256: sha256(readFileSync(scriptPath, 'utf8')),
    generated_at: new Date().toISOString(),
    live_tested: false,
    synthetic: true,
    license: 'Apache-2.0',
    encoding: { compression: 'none', packed_chunks: false },
    session_jsonl_sha256: sha256(jsonl),
  }
  writeFileSync(join(out, 'meta.json'), `${JSON.stringify(meta, null, 2)}\n`)
  console.log(`wrote ${events.length} events, ${requests.length} requests, ${closers.length} closers to ${out}`)
}

main().catch((error: unknown) => {
  console.error(error)
  process.exit(1)
})
