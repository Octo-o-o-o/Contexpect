# Implementation Handoff Prompt

I have a complete image-based UI design pack at /Users/wangyixiao/WorkSpace/ContextView/docs/gpt-img-2-design/20260904-1200-contexpect-doctor. Please read every markdown file and every image in that folder before editing code.

Target project: /Users/wangyixiao/WorkSpace/ContextView
Goal: build the new Contexpect product described by the canonical PRD and implement this design pack as its first production UI. This is a greenfield implementation, not a refactor, and there is no existing product behavior to preserve.

First, read the canonical PRD and agent expansion research, then inspect the repository structure. If the codebase is still empty, create an explicit architecture/acceptance plan before writing product code; do not invent a UI-only mock that bypasses the Receipt/evidence contracts.

Then design the implementation plan in this order:
1. Global structure and design tokens.
2. Shared layout and navigation.
3. Shared components.
4. Page-by-page implementation.
5. Responsive and state behavior.
6. Screenshot verification.

Before building all routes, implement the Doctor screen against deterministic fixture Receipts. Open it with the browser tool, screenshot it, compare it against the target design image, and revise tokens/components if the demo exposes mismatches. The Markdown screen spec is canonical for text and semantics; the image is canonical for composition and visual direction.

Use the architecture selected and approved from the product requirements. Keep static resolution, native evidence, user-attested evidence, Unknown and Indeterminate distinct in both types and UI. Do not implement a health score or a direct Fix action.

After implementation, run the app locally with fixture and live-adapter data. Use the browser tool to screenshot every implemented desktop page and compare against the design pack. Verify the 18-family 4/9/5 fixture programmatically rather than trusting pixels. Run the PRD's semantic, security and adapter conformance gates in addition to visual checks.

When done, update /Users/wangyixiao/WorkSpace/ContextView/docs/gpt-img-2-design/20260904-1200-contexpect-doctor/06-implementation-plan.md with what was implemented, verification screenshots, remaining differences, and follow-up risks. Commit and push only if I explicitly ask for that in this task.
