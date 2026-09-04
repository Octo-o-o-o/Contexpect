# Image Prompts

> 三轮主设计探索之后有两次 validation patch：一次来自本机 agent 的证据语义审查，一次来自互联网/本机适配器范围调研。后两次不改变布局方向，只修正事实和标签。前三轮 prompt 为依据当时调用内容整理的 canonical reconstruction；最终 scope patch 保存为实际调用原文。

## Prompt Log

| Image | Round | Purpose |
| --- | --- | --- |
| `reviews/round-1-context-doctor.png` | 1 | 建立 clinical observability cockpit 和证据优先心智 |
| `reviews/round-2-context-doctor.png` | 2 | 增加 symptom-first input、six facets、coverage taxonomy |
| `reviews/round-3-pre-validation.png` | 3 | 补全 coordinate、Assets IA、evidence-first CTA 和 treatment gate |
| `reviews/post-agent-validation-scope13.png` | validation | 修复 Resolved/Observed/Not exposed 的误导语义 |
| `images/10-context-doctor-final.png` | scope validation | 将适配器范围从 13 个示例校正为 18 个 family |

## Round 1 — canonical reconstruction

```text
Create a high-fidelity desktop app screen for “Contexpect”, a local-first AI coding context observability and control product. Draw the Context Doctor diagnosis cockpit, not a marketing page. Use a calm clinical observability visual language: dark navy navigation shell, warm off-white content, crisp borders, restrained teal/amber/coral/violet semantic colors, no gradients, no medical illustration, no health score.

At 1440×900 proportions, show project/device/snapshot coordinates, a multi-axis status summary, a dense findings list, and a right-side evidence/care panel. Include Codex, Claude, Cursor and Grok plus chips for other coding agents. The selected finding should be about a Cursor user rule whose model visibility cannot be proven. Show an evidence chain and safe treatment plan; no immediate destructive fix. Make it look implementation-ready and legible.
```

## Round 2 — canonical reconstruction

```text
Refine the previous Contexpect Context Doctor desktop screenshot without changing its quiet clinical design. Add a symptom-first input (“Describe what feels wrong…”) and quick symptom chips. Separate finding Evidence from Impact so Unknown is not presented as severity. Add the six independent context facets: Installed, Discoverable, Eligible, Visible, Use evidence, Effect. Show a truthful chain from source through resolver and model-visible claim to native observation. Add adapter coverage groups including Coze. Correct the Cursor example so the source can exist while native model visibility remains unavailable. Keep the screen dense but calm, accessible, and production-ready.
```

## Round 3 — canonical reconstruction

```text
Produce the third and final architecture refinement of the Contexpect Context Doctor screen. Preserve the established navy/warm-white visual system. Fix adapter counts, add account/policy to the coordinate bar, add Assets navigation, make “Collect evidence” the primary CTA, and lock Treatment until the claim is verified. Group adapter chips by evidence capability instead of online/offline. Keep Confirmed, Suspected and Unknown as separate counts, never a health score. Keep all actions previewable and reversible. The selected Cursor finding must remain Indeterminate because the native surface is absent. Render a polished 1440×900 desktop app screenshot.
```

## Agent-validation semantic patch — canonical reconstruction

```text
Surgically edit the round-three screenshot. Preserve layout and styling. Replace ambiguous “Resolved only” with “Static resolution only” and use a blue document icon. Replace any pass-like native observation for an unavailable surface with a neutral gray “Not exposed” state. The finding evidence must say “Static resolution”, not Observed. Add the plain-language explanation “The rule exists, but Cursor cannot prove the model received it.” Spell “DeepSeek DSH” clearly. Preserve the locked Treatment card and evidence-first CTA.
```

## Final scope-validation patch — actual prompt

```text
Edit this existing high-fidelity desktop product UI screenshot with a surgical post-research scope correction. Preserve the exact Contexpect visual design, layout, typography, dark navy shell, warm off-white surfaces, diagnosis content, evidence chain, six context facets, navigation, CTA hierarchy, and all other copy. Do not redesign the page.

Only update the adapter coverage information so the screenshot visibly communicates that the product mechanism tracks 18 adapters, not only 13:
1) In the top coverage summary, replace the current “9 / 13 agent surfaces have native evidence” wording with the truthful fixture text “4 native-evidence · 9 static-only · 5 need connector”. Keep the visual weight calm and compact.
2) In the lower-right “Adapter coverage” area, retain the three semantic groups and their distinct evidence colors/icons:
- Native evidence (teal check): Codex, Claude, Grok, DeepSeek DSH.
- Static resolution only (blue document icon): Cursor, OpenCode, Kimi Code, ZCode, Gemini CLI, Qwen Code, Goose, Copilot, Kiro.
- Needs connector (violet question icon): Coze, Cline, Aider, OpenHands, Windsurf.
Fit all 18 names legibly as compact wrapping chips without expanding or overlapping the panel. If space is tight, tighten chip padding and vertical gaps, not font readability. Use “DeepSeek DSH” exactly.
3) Preserve the finding evidence label “Static resolution”, the neutral gray “Not exposed” native observation, the plain-language diagnosis “The rule exists, but Cursor cannot prove the model received it.”, and the locked Treatment card.
4) Keep the title Context Doctor, all counts and other page sections unchanged. Do not introduce a health score, gradients, fake charts, or medical illustrations.

The final output must look like a polished production desktop app screenshot at 1440×900 proportions, crisp and legible.
```
