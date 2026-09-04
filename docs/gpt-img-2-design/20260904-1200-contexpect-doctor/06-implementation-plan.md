# Implementation Plan

> 本文件是设计到代码的实施映射，不表示已开始开发。当前仓库仍只有方案与设计资产。

## Code Mapping

| Design Area | Existing File/Component | Planned Change |
| --- | --- | --- |
| App shell | 无 | Tauri window + React `AppShell`、`SideNav`、`CoordinateBar` |
| Theme/tokens | `02-design-system.md` | CSS custom properties + typed semantic token map；evidence 与 severity token 分离 |
| Doctor route | 无 | `/doctor` route、query/filter state、selected finding drawer |
| Receipt data | PRD §4/§10 | Rust Context IR/Receipt schema 生成 typed frontend DTO |
| Finding list | 无 | virtualizable accessible table/list，默认 impact/severity priority |
| Evidence chain | 无 | 统一消费 claim/evidence，不在 UI 自行推断 truth |
| Context facets | 无 | 六 facet component；Unknown/Indeterminate 独立 |
| Adapter coverage | 18-family research matrix | family/version/surface/capability drill-down，三类 summary 由 Receipt 计算 |
| Care Plan | PRD F-09 | read-only suggestion → preview → authority → authorization → apply → recheck |
| Empty/error | 无 | no-findings-with-coverage、permission、auth、sandbox、unsupported version、connector missing |

## Architecture Constraints

- UI 不直接扫描 home、解释规则或调用 harness；全部通过同一个 local core API/Receipt。
- `Static resolution` 必须来自 versioned adapter resolver；`Native evidence` 必须带 provenance 和 declared coverage。
- 每个 UI 状态都绑定 stable reason code，不根据字符串错误消息临时分类。
- Adapter family、surface、version、installation 和 connection 分开建模；不能用一个 `enabled` 布尔值代替。
- Desktop、CLI、daemon、CI 输出使用同一 claim vocabulary 和 Receipt schema。

## Phases

1. Freeze schemas、adapter manifest、18-family compatibility matrix、reason codes 和 fixture contract。
2. Build Rust core、four anchor adapters、Receipt/Evidence ledger 和 local API。
3. Build tokens、AppShell、CoordinateBar、semantic status components。
4. Implement Doctor route with deterministic fixture data and every empty/error state。
5. Connect Doctor to live core；ensure UI does not upgrade static claims to observed。
6. Implement expansion adapters and Coze connector according to the research matrix。
7. Implement Care Plan/preview/authority/rollback/recheck and Receipt navigation。
8. Add Monitor、Assets、Integrations、Policy、Standards and remaining product routes。
9. Run desktop responsive、keyboard、screen-reader、privacy/redaction and screenshot gates。
10. Run native-oracle conformance、cross-device、daemon/CI and full-product acceptance gates from the PRD。

## Verification Targets

| Page | Mock Data | Screenshot Path | Status |
| --- | --- | --- | --- |
| Doctor 1440×900 | `OctoWorkflow` fixture；4 findings；18 adapters | `verification/doctor-1440x900.png` | not started |
| Doctor 1024×768 | same；drawer overlay | `verification/doctor-1024x768.png` | not started |
| Doctor empty | zero findings + Unknown coverage | `verification/doctor-empty.png` | not started |
| Doctor adapter failure | sandbox/model/auth/connector reason fixtures | `verification/doctor-adapter-errors.png` | not started |
| Care Plan locked | selected Indeterminate claim | `verification/care-plan-locked.png` | not started |
| Treatment preview | explicit authority/loss/rollback fixture | `verification/treatment-preview.png` | not started |

Design comparison gate:

- Browser screenshot at 1440×900 must be compared side-by-side with `images/10-context-doctor-final.png`.
- Required semantic copy and counts come from `03-screen-specs.md`, not OCR from the generated image.
- Verify 18 names programmatically in DOM and confirm 4/9/5 sum to 18.
- Automated accessibility scan is necessary but not sufficient；keyboard/focus/read-order must be manually exercised。

## Risks

- Image reference is not pixel-spec truth for tiny copy；Markdown specs are canonical。
- 18 adapters can dominate schedule；shared SDK/fixtures must land before per-family work to avoid 18 bespoke implementations。
- Native surface changes rapidly；unknown fail-closed and signed/versioned corpus are product requirements, not polish。
- IDE-only products may expose no safe runtime evidence；complete behavior is honest `required-unknown-honesty`, not proxy interception by default。
- Adapter startup errors can contain secrets or absolute paths；redaction happens in core before UI/log persistence。
- Tauri WebView/font rendering differs across macOS/Windows/Linux；visual regression tolerances must be semantic + regional, not raw whole-image equality。
