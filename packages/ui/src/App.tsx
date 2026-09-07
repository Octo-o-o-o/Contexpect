import { NavLink, Navigate, Route, Routes, useLocation, useParams } from "react-router-dom";
import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type KeyboardEvent } from "react";
import { NAV } from "./routes";
import { t, type Locale } from "./i18n";
import { asObj, postJson, requestJson, type Json } from "./api";
import { projectFieldValue, projectVisible, revealed } from "./mask";
import { classifyFailure, classifyPayload } from "./page-state";
import { stateApplies } from "./page-contract";
import { changedFields, projectToSchema, setPath, validateDraft } from "./settings-form";

const FACETS = [
  "installed",
  "discoverable",
  "eligible",
  "model-visible",
  "use-evidence",
  "outcome-affecting",
] as const;

export function App() {
  const [locale, setLocale] = useState<Locale>("zh-CN");
  const [privacy, setPrivacy] = useState<"default" | "screenshot">("default");
  const [project, setProject] = useState("");
  const [receipt, setReceipt] = useState<Json>({});
  const [doctor, setDoctor] = useState<Json>({});
  const [status, setStatus] = useState<"idle" | "loading" | "error" | "stale">("idle");
  const [error, setError] = useState("");
  // The C04 verdict for the shared inspect/doctor request, so Checkup,
  // Inspector and Doctor render the same banner as every other page instead
  // of a stringified exception.
  const [verdict, setVerdict] = useState<StateVerdict | null>(null);
  const [lastSymptom, setLastSymptom] = useState<string | undefined>(undefined);
  const [hold, setHold] = useState(false);
  const loc = useLocation();
  const isRevealed = revealed(hold, privacy);
  const showProject = projectVisible(privacy, hold);

  function confirmCopy(event: ClipboardEvent) {
    if (!isRevealed) {
      event.preventDefault();
      return;
    }
    if (!window.confirm(t(locale, "copyConfirm"))) {
      event.preventDefault();
    }
  }

  async function runInspect(symptom?: string) {
    if (!project) {
      setError(t(locale, "projectRequired"));
      setStatus("error");
      setVerdict({ state: "error", reasonCode: "ui.project_required", retryable: false });
      return;
    }
    setStatus("loading");
    setError("");
    setVerdict(null);
    setLastSymptom(symptom);

    const inspected = await requestJson("/api/v1/inspect", {
      method: "POST",
      body: JSON.stringify({ project, harness: "codex", symptom: symptom ?? "" }),
    });
    if (!inspected.ok) {
      // The reason code survives instead of being folded into a message.
      const failed = classifyFailure(inspected.kind, inspected.code);
      setStatus("error");
      setError(inspected.message || inspected.code);
      setVerdict({ ...failed, state: failed.state });
      return;
    }
    const out = asObj(inspected.data);
    setReceipt(asObj(out.receipt));
    setStatus(out.stale === true ? "stale" : "idle");

    const query = symptom ? `?symptom=${encodeURIComponent(symptom)}` : "";
    const diagnosed = await requestJson(`/api/v1/doctor${query}`);
    if (!diagnosed.ok) {
      const failed = classifyFailure(diagnosed.kind, diagnosed.code);
      setStatus("error");
      setError(diagnosed.message || diagnosed.code);
      setVerdict({ ...failed, state: failed.state });
      return;
    }
    const doc = asObj(diagnosed.data);
    setDoctor(doc);
    // Classify the diagnosis itself: Unknown cells make it partial, and a
    // stale Receipt stays stale.
    const payload = classifyPayload(out.stale === true ? { ...doc, stale: true } : doc);
    setVerdict({ ...payload, state: payload.state });
  }

  function onHoldKey(event: KeyboardEvent<HTMLButtonElement>, down: boolean) {
    if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      setHold(down);
    }
  }

  const findings = useMemo(() => {
    const list = doctor.findings;
    return Array.isArray(list) ? (list as Json[]) : [];
  }, [doctor]);

  return (
    <div className="shell" data-privacy={privacy} lang={locale}>
      <nav className="nav" aria-label="primary">
        <div className="brand">Contexpect</div>
        {NAV.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) => (isActive ? "active" : "")}
            // Below 1024px the label is collapsed visually. An explicit name
            // keeps the link identifiable to assistive tech regardless of how
            // a given AT treats visually-hidden text (WCAG 2.2 SC 2.4.4, 4.1.2).
            aria-label={t(locale, item.key)}
          >
            <span>{t(locale, item.key)}</span>
          </NavLink>
        ))}
      </nav>
      <div className="main">
        <div className="topbar">
          <label>
            {t(locale, "project")}
            <input
              value={projectFieldValue(project, showProject)}
              onChange={(e) => setProject(e.target.value)}
              onCopy={confirmCopy}
              data-mask
              data-revealed={showProject ? "true" : "false"}
              // Read-only while masked: otherwise editing writes the mask
              // characters back into the real project path. `aria-hidden`
              // must not appear on a focusable element, so the masked state
              // is announced through the label instead of hiding the field.
              readOnly={!showProject}
              aria-label={showProject ? t(locale, "project") : t(locale, "masked")}
              placeholder="<project>"
            />
          </label>
          <button className="primary" onClick={() => void runInspect()}>
            {t(locale, "inspect")}
          </button>
          <span className="pill">{status === "loading" ? t(locale, "loading") : status}</span>
          <label>
            {t(locale, "privacy")}
            <select
              value={privacy}
              onChange={(e) => setPrivacy(e.target.value as "default" | "screenshot")}
            >
              <option value="default">{t(locale, "privacyDefault")}</option>
              <option value="screenshot">{t(locale, "privacyScreenshot")}</option>
            </select>
          </label>
          <select value={locale} onChange={(e) => setLocale(e.target.value as Locale)} aria-label="locale">
            <option value="zh-CN">简体中文</option>
            <option value="en">English</option>
          </select>
          <button
            type="button"
            className="secondary"
            tabIndex={0}
            onMouseDown={() => setHold(true)}
            onMouseUp={() => setHold(false)}
            onMouseLeave={() => setHold(false)}
            onKeyDown={(e) => onHoldKey(e, true)}
            onKeyUp={(e) => onHoldKey(e, false)}
            onBlur={() => setHold(false)}
            aria-pressed={hold}
          >
            {t(locale, "unmaskHold")}
          </button>
          <span className="muted">{t(locale, "egressSeparate")}</span>
        </div>
        {error ? <p role="alert">{error}</p> : null}
        <div className="workspace">
          <Routes>
            <Route path="/" element={<Navigate to="/doctor" replace />} />
            <Route
              path="/doctor"
              element={
                <DoctorPage
                  locale={locale}
                  project={project}
                  findings={findings}
                  doctor={doctor}
                  receipt={receipt}
                  hold={isRevealed}
                  status={status}
                  error={error}
                  verdict={verdict}
                  onRetry={() => void runInspect(lastSymptom)}
                  onDiagnose={(symptom) => void runInspect(symptom)}
                  onCopy={confirmCopy}
                />
              }
            />
            <Route
              path="/checkup"
              element={
                <CheckupPage
                  receipt={receipt}
                  status={status}
                  error={error}
                  verdict={verdict}
                  onRetry={() => void runInspect(lastSymptom)}
                  locale={locale}
                />
              }
            />
            <Route
              path="/inspector"
              element={
                <InspectorPage
                  receipt={receipt}
                  hold={isRevealed}
                  locale={locale}
                  verdict={verdict}
                  onRetry={() => void runInspect(lastSymptom)}
                  onCopy={confirmCopy}
                />
              }
            />
            <Route path="/compare" element={<ComparePage locale={locale} />} />
            <Route path="/receipts" element={<StateView path="/api/v1/receipts" route="/receipts" title={t(locale, "receipts")} locale={locale} />} />
            <Route path="/receipts/:id" element={<ReceiptDetail locale={locale} />} />
            <Route path="/assets" element={<AssetsPage locale={locale} />} />
            <Route path="/assets/:id" element={<EntityPage folder="assets" locale={locale} />} />
            <Route path="/sessions" element={<StateView path="/api/v1/sessions" route="/sessions" title={t(locale, "sessions")} locale={locale} />} />
            <Route path="/sessions/:id" element={<EntityPage folder="sessions" locale={locale} />} />
            <Route path="/monitor" element={<StateView path="/api/v1/monitor" route="/monitor" title={t(locale, "monitor")} locale={locale} />} />
            <Route path="/lab" element={<StateView path="/api/v1/lab" route="/lab" title={t(locale, "lab")} locale={locale} />} />
            <Route path="/lab/:id" element={<EntityPage folder="lab" locale={locale} />} />
            <Route path="/sync" element={<SyncPage locale={locale} />} />
            <Route path="/policy" element={<StateView path="/api/v1/policy" route="/policy" title={t(locale, "policy")} locale={locale} />} />
            <Route path="/standards" element={<StateView path="/api/v1/standards" route="/standards" title={t(locale, "standards")} locale={locale} />} />
            <Route path="/standards/:id" element={<EntityPage folder="standards" locale={locale} />} />
            <Route path="/settings" element={<SettingsPage locale={locale} />} />
            <Route path="/exceptions" element={<StateView path="/api/v1/exceptions" route="/exceptions" title={t(locale, "exceptions")} locale={locale} />} />
            <Route path="/team/compliance" element={<StateView path="/api/v1/team/compliance" route="/team/compliance" title={t(locale, "team")} locale={locale} />} />
            <Route path="/care-plan/:findingId" element={<CarePlanPage locale={locale} />} />
            <Route path="/integrations" element={<IntegrationsPage locale={locale} />} />
            <Route path="/integrations/:id" element={<EntityPage folder="integrations" locale={locale} />} />
            <Route path="*" element={<p>{t(locale, "notFound")}: {loc.pathname}</p>} />
          </Routes>
        </div>
      </div>
    </div>
  );
}

function EvidencePill({ kind, locale }: { kind: "native" | "static" | "unknown" | "attested"; locale: Locale }) {
  const label = t(locale, kind);
  return (
    <span className={`pill ${kind === "attested" ? "native" : kind}`}>
      <span aria-hidden>{kind === "unknown" ? "?" : kind === "native" ? "✓" : "i"}</span>
      {label}
    </span>
  );
}

function MaskedText({
  text,
  hold,
  locale,
  onCopy,
}: {
  text: string;
  hold: boolean;
  locale: Locale;
  onCopy: (event: ClipboardEvent) => void;
}) {
  const display = hold ? text : "••••";
  return (
    <pre
      className="mono"
      data-mask
      data-revealed={hold ? "true" : "false"}
      aria-label={hold ? undefined : t(locale, "masked")}
      onCopy={onCopy}
    >
      {display}
    </pre>
  );
}

function DoctorPage({
  locale,
  project,
  findings,
  doctor,
  receipt,
  hold,
  status,
  error,
  verdict,
  onRetry,
  onDiagnose,
  onCopy,
}: {
  locale: Locale;
  project: string;
  findings: Json[];
  doctor: Json;
  receipt: Json;
  hold: boolean;
  status: string;
  error: string;
  verdict: StateVerdict | null;
  onRetry: () => void;
  onDiagnose: (symptom: string) => void;
  onCopy: (event: ClipboardEvent) => void;
}) {
  const [selected, setSelected] = useState(0);
  const [symptom, setSymptom] = useState("");
  const [collectStatus, setCollectStatus] = useState("");
  const [collectError, setCollectError] = useState("");
  const [collectResult, setCollectResult] = useState<Json>({});
  const counts = asObj(doctor.counts);
  const finding = findings[selected];

  /**
   * Change the drawer's subject.
   *
   * Refused while a collection is in flight: the request was issued for the
   * current finding, and letting the selection move would attach its result
   * to a different one.
   */
  function selectFinding(index: number) {
    if (collectStatus === "loading") return;
    setSelected(index);
  }
  const facets = asObj(receipt.facets);
  const canAct = project.trim().length > 0;
  const diagnosing = status === "loading";

  async function collectEvidence() {
    if (!canAct) return;
    setCollectStatus("loading");
    setCollectError("");
    try {
      const out = asObj(await postJson("/api/v1/collect", { project }));
      setCollectResult(out);
      setCollectStatus("idle");
    } catch (err) {
      setCollectStatus("error");
      setCollectError(String(err));
    }
  }

  return (
    <>
      <section className="panel">
        <h1>{t(locale, "contextDoctor")}</h1>
        <div className="row">
          <input
            style={{ flex: 1 }}
            value={symptom}
            onChange={(e) => setSymptom(e.target.value)}
            placeholder={t(locale, "symptom")}
            aria-label="symptom"
          />
          <button
            type="button"
            className="primary"
            disabled={!canAct || diagnosing}
            onClick={() => onDiagnose(symptom)}
            title={canAct ? undefined : t(locale, "diagnoseDisabled")}
          >
            {diagnosing ? t(locale, "loading") : t(locale, "diagnose")}
          </button>
        </div>
        {!canAct ? <p className="muted">{t(locale, "diagnoseDisabled")}</p> : null}
        <SharedStateBanner route="/doctor" verdict={verdict} locale={locale} onRetry={onRetry} />
        {error ? <p role="alert">{error}</p> : null}
        {doctor.symptom ? (
          <p className="muted">
            {t(locale, "symptom")} {String(doctor.symptom)}
          </p>
        ) : null}
        <div className="row" style={{ marginTop: 12 }}>
          <span className="pill confirmed">
            {String(counts.confirmed ?? 0)} {t(locale, "confirmed")}
          </span>
          <span className="pill suspected">
            {String(counts.suspected ?? 0)} {t(locale, "suspected")}
          </span>
          <span className="pill unknown">
            {String(counts.unknown ?? 0)} {t(locale, "unknown")}
          </span>
        </div>
        <p className="muted">{String(counts.note ?? "")}</p>
        {findings.length === 0 ? (
          <p>{t(locale, "emptyFindings")}</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>{t(locale, "finding")}</th>
                <th>{t(locale, "affectedSurfaces")}</th>
                <th>{t(locale, "evidence")}</th>
                <th>{t(locale, "impact")}</th>
                <th>{t(locale, "firstSeen")}</th>
              </tr>
            </thead>
            <tbody>
              {findings.map((item, index) => (
                <tr
                  key={String(item.finding_id)}
                  tabIndex={0}
                  // Selection is announced, so the drawer's content change is
                  // not silent for assistive tech.
                  aria-selected={index === selected}
                  // While a collection is running the selection is frozen:
                  // switching would land the result on a different finding.
                  aria-disabled={collectStatus === "loading"}
                  onClick={() => selectFinding(index)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      selectFinding(index);
                    }
                  }}
                  style={{ background: index === selected ? "#eef3f6" : undefined }}
                >
                  <td>{String(item.title)}</td>
                  <td>{JSON.stringify(item.affected_surfaces)}</td>
                  <td>
                    <EvidencePill
                      locale={locale}
                      kind={item.evidence_state === "indeterminate" ? "unknown" : "static"}
                    />
                  </td>
                  <td>{String(item.impact)}</td>
                  <td>{String(item.first_seen)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <h2>{t(locale, "adapterCoverage")}</h2>
        <AdapterCoverage locale={locale} />
      </section>
      <aside
        className="panel drawer"
        // A persistent region rather than a modal: it has no open/close and
        // therefore no focus to return or Esc to handle. What it does need is
        // for its content changes to be perceivable and its busy state known.
        role="region"
        aria-labelledby="drawer-title"
        aria-busy={collectStatus === "loading"}
      >
        <h2 id="drawer-title">{t(locale, "diagnosisEvidence")}</h2>
        {collectStatus === "loading" ? (
          <p role="status">{t(locale, "drawerBusy")}</p>
        ) : null}
        {finding ? (
          <>
            <p>
              {t(locale, "decision")}:{" "}
              <strong>
                {asObj(finding.treatment).locked === true
                  ? t(locale, "decisionIndeterminate")
                  : t(locale, "decisionOpen")}
              </strong>
            </p>
            <EvidenceChain locale={locale} />
            <h2>{t(locale, "sixFacets")}</h2>
            <div className="facet-grid">
              {FACETS.map((name) => {
                const claim = asObj(facets[name]);
                const truth = String(claim.truth_state ?? "unknown");
                return (
                  <div key={name} className={truth === "indeterminate" ? "facet broken" : "facet"}>
                    <div>{name}</div>
                    <div>{truth}</div>
                  </div>
                );
              })}
            </div>
            <p>
              <button
                type="button"
                className="primary"
                disabled={!canAct || collectStatus === "loading"}
                onClick={() => void collectEvidence()}
                title={canAct ? undefined : t(locale, "collectDisabled")}
              >
                {collectStatus === "loading" ? t(locale, "loading") : t(locale, "collectEvidence")}
              </button>
            </p>
            {!canAct ? <p className="muted">{t(locale, "collectDisabled")}</p> : null}
            {collectError ? <p role="alert">{collectError}</p> : null}
            {collectResult.inventory_digest ? (
              <pre className="mono">{JSON.stringify(collectResult, null, 2)}</pre>
            ) : null}
            <p className="muted">
              {t(locale, "treatmentLocked")}: {String(asObj(finding.treatment).locked === true)}. {t(locale, "cannotUnlock")}
            </p>
            <MaskedText text={String(finding.reason_code)} hold={hold} locale={locale} onCopy={onCopy} />
          </>
        ) : (
          <p className="muted">{t(locale, "selectFinding")}</p>
        )}
      </aside>
    </>
  );
}

function EvidenceChain({ locale }: { locale: Locale }) {
  const steps = [
    [t(locale, "evidenceDeclared"), t(locale, "evidenceDeclaredState")],
    [t(locale, "evidenceResolver"), t(locale, "evidenceResolverState")],
    [t(locale, "evidenceModelVisible"), t(locale, "evidenceModelVisibleState")],
    [t(locale, "evidenceNative"), t(locale, "evidenceNativeState")],
  ];
  return (
    <ol>
      {steps.map(([a, b]) => (
        <li key={a}>
          {a}: {b}
        </li>
      ))}
    </ol>
  );
}

function AdapterCoverage({ locale }: { locale: Locale }) {
  const [data, setData] = useState<Json>({});
  const [failure, setFailure] = useState("");
  useEffect(() => {
    void requestJson("/api/v1/integrations").then((result) => {
      // Swallowing the failure would render an empty coverage row that is
      // indistinguishable from "no families", which is a different claim.
      if (result.ok) setData(asObj(result.data));
      else setFailure(result.code);
    });
  }, []);
  const families = Array.isArray(data.families) ? (data.families as Json[]) : [];
  if (failure) {
    return (
      <p role="alert">
        {t(locale, "reasonCodeLabel")}: <code>{failure}</code>
      </p>
    );
  }
  return (
    <div className="row" data-testid="adapter-coverage">
      {families.map((f) => (
        <span key={String(f.family_id)} className="pill" data-family={String(f.family_id)}>
          {String(f.family_name)}
        </span>
      ))}
    </div>
  );
}

function CheckupPage({
  receipt,
  status,
  error,
  verdict,
  onRetry,
  locale,
}: {
  receipt: Json;
  status: string;
  error: string;
  verdict: StateVerdict | null;
  onRetry: () => void;
  locale: Locale;
}) {
  return (
    <section className="panel">
      <h1>{t(locale, "checkup")}</h1>
      <p>{t(locale, "checkupLead")}</p>
      <p>
        {t(locale, "statusLabel")}: {status}
      </p>
      <SharedStateBanner route="/checkup" verdict={verdict} locale={locale} onRetry={onRetry} />
      {error ? <p role="alert">{error}</p> : null}
      <pre className="mono">{JSON.stringify(receipt.policy_result ?? {}, null, 2)}</pre>
    </section>
  );
}

function InspectorPage({
  receipt,
  hold,
  locale,
  verdict,
  onRetry,
  onCopy,
}: {
  receipt: Json;
  hold: boolean;
  locale: Locale;
  verdict: StateVerdict | null;
  onRetry: () => void;
  onCopy: (event: ClipboardEvent) => void;
}) {
  const facets = asObj(receipt.facets);
  const budget = asObj(receipt.budget);
  return (
    <section className="panel">
      <h1>{t(locale, "inspector")}</h1>
      <p>
        <a href="#why">{t(locale, "whyHere")}</a> · <a href="#how">{t(locale, "howKnow")}</a>
      </p>
      <SharedStateBanner route="/inspector" verdict={verdict} locale={locale} onRetry={onRetry} />
      <div className="facet-grid">
        {FACETS.map((name) => {
          const claim = asObj(facets[name]);
          return (
            <div key={name} className="facet">
              <h2>{name}</h2>
              <div>
                {t(locale, "truthState")}: {String(claim.truth_state ?? "unknown")}
              </div>
              <div>
                {t(locale, "provenance")}: {String(claim.provenance ?? "")}
              </div>
              <div>
                {t(locale, "coverage")}: {String(claim.coverage ?? "")}
              </div>
              <div>
                {t(locale, "precision")}: {String(claim.precision ?? "")}
              </div>
              <div>
                {t(locale, "knowledgeStatus")}: {String(claim.knowledge_status ?? "")}
              </div>
              <EvidencePill
                locale={locale}
                kind={
                  String(claim.provenance).startsWith("native")
                    ? "native"
                    : String(claim.truth_state) === "indeterminate"
                      ? "unknown"
                      : "static"
                }
              />
            </div>
          );
        })}
      </div>
      <h2>{t(locale, "budget")}</h2>
      <p className="muted">{t(locale, "budgetUnknownNote")}</p>
      <div id="how">
        <MaskedText text={JSON.stringify(budget, null, 2)} hold={hold} locale={locale} onCopy={onCopy} />
      </div>
      <h2 id="why">{t(locale, "whyHere")}</h2>
      <pre className="mono">{JSON.stringify(receipt.explanation ?? [], null, 2)}</pre>
    </section>
  );
}

function ComparePage({ locale }: { locale: Locale }) {
  const [receipts, setReceipts] = useState<Json[]>([]);
  const [a, setA] = useState("");
  const [b, setB] = useState("");
  const [diff, setDiff] = useState<unknown>(null);
  const [err, setErr] = useState("");
  const [loading, setLoading] = useState(false);
  const [verdict, setVerdict] = useState<StateVerdict | null>(null);

  useEffect(() => {
    void requestJson("/api/v1/receipts").then((result) => {
      if (!result.ok) {
        setErr(result.message || result.code);
        setVerdict(classifyFailure(result.kind, result.code));
        return;
      }
      const list = asObj(result.data).receipts;
      setReceipts(Array.isArray(list) ? (list as Json[]) : []);
      // Fewer than two Receipts is an empty compare, not a broken one.
      setVerdict(classifyPayload(result.data));
    });
  }, []);

  async function runDiff() {
    if (!a || !b) return;
    setLoading(true);
    setErr("");
    const result = await requestJson(`/api/v1/diff?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);
    if (result.ok) {
      setDiff(result.data);
      setVerdict(classifyPayload(result.data));
    } else {
      setErr(result.message || result.code);
      setVerdict(classifyFailure(result.kind, result.code));
    }
    setLoading(false);
  }
  const ids = receipts
    .map((item) => String(item.receipt_id ?? ""))
    .filter((id) => id.length > 0);
  return (
    <section className="panel">
      <h1>{t(locale, "compare")}</h1>
      {ids.length < 2 ? <p className="muted">{t(locale, "compareNeedTwo")}</p> : null}
      <SharedStateBanner
        route="/compare"
        verdict={verdict}
        locale={locale}
        onRetry={() => void runDiff()}
      />
      {err ? <p role="alert">{err}</p> : null}
      <div className="row">
        <select value={a} onChange={(e) => setA(e.target.value)} aria-label="diff-a">
          <option value="">{t(locale, "empty")}</option>
          {ids.map((id) => (
            <option key={`a-${id}`} value={id}>
              {id}
            </option>
          ))}
        </select>
        <select value={b} onChange={(e) => setB(e.target.value)} aria-label="diff-b">
          <option value="">{t(locale, "empty")}</option>
          {ids.map((id) => (
            <option key={`b-${id}`} value={id}>
              {id}
            </option>
          ))}
        </select>
        <button type="button" className="primary" disabled={!a || !b || loading} onClick={() => void runDiff()}>
          {loading ? t(locale, "loading") : t(locale, "runDiff")}
        </button>
      </div>
      {diff ? <pre className="mono">{JSON.stringify(diff, null, 2)}</pre> : null}
    </section>
  );
}

/** i18n key for each C04 state's label and its next step. */
const STATE_LABEL: Record<string, string> = {
  empty: "stateEmpty",
  error: "stateError",
  partial: "statePartial",
  stale: "stateStale",
  offline: "stateOffline",
  "permission-denied": "statePermissionDenied",
  "unsupported-version": "stateUnsupportedVersion",
  "connector-missing": "stateConnectorMissing",
};
const STATE_NEXT: Record<string, string> = {
  empty: "nextEmpty",
  error: "nextError",
  partial: "nextPartial",
  stale: "nextStale",
  offline: "nextOffline",
  "permission-denied": "nextPermissionDenied",
  "unsupported-version": "nextUnsupportedVersion",
  "connector-missing": "nextConnectorMissing",
};

type StateVerdict = { state: string; reasonCode: string; retryable: boolean };

type Resource = {
  status: string;
  reasonCode: string;
  retryable: boolean;
  data: unknown;
  retry: () => void;
  cancel: () => void;
};

/**
 * Fetch one resource and classify the outcome into a C04 state.
 *
 * The in-flight request is abortable, so "cancel" actually stops the work
 * rather than only hiding it. Retry re-sends the same request and changes no
 * parameter, so it cannot widen an authorization that was refused.
 */
function useResource(path: string): Resource {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<Omit<Resource, "retry" | "cancel">>({
    status: "loading",
    reasonCode: "",
    retryable: false,
    data: null,
  });
  const controller = useRef<AbortController | null>(null);

  useEffect(() => {
    const ctrl = new AbortController();
    controller.current = ctrl;
    setState({ status: "loading", reasonCode: "", retryable: false, data: null });
    void requestJson(path, { signal: ctrl.signal }).then((result) => {
      if (result.ok) {
        const verdict = classifyPayload(result.data);
        setState({ ...verdict, status: verdict.state, data: result.data });
        return;
      }
      // A cancelled request is the user's own doing, not an unreachable
      // daemon, so it does not present as offline.
      if (result.code === "api.cancelled") {
        setState({ status: "cancelled", reasonCode: result.code, retryable: true, data: null });
        return;
      }
      const verdict = classifyFailure(result.kind, result.code);
      setState({ ...verdict, status: verdict.state, data: null });
    });
    return () => ctrl.abort();
  }, [path, attempt]);

  return {
    ...state,
    retry: () => setAttempt((n) => n + 1),
    cancel: () => controller.current?.abort(),
  };
}

/**
 * Render a verdict that was computed elsewhere (the shared inspect/doctor
 * request), so those pages show the same banner as the self-fetching ones.
 */
function SharedStateBanner({
  route,
  verdict,
  locale,
  onRetry,
}: {
  route: string;
  verdict: StateVerdict | null;
  locale: Locale;
  onRetry: () => void;
}) {
  if (!verdict) return null;
  return (
    <StateBanner
      route={route}
      status={verdict.state}
      reasonCode={verdict.reasonCode}
      retryable={verdict.retryable}
      locale={locale}
      onRetry={onRetry}
    />
  );
}

/** The banner that states which C04 state a page is in, and what to do. */
function StateBanner({
  route,
  status,
  reasonCode,
  retryable,
  locale,
  onRetry,
}: {
  route: string;
  status: string;
  reasonCode: string;
  retryable: boolean;
  locale: Locale;
  onRetry: () => void;
}) {
  const labelKey = STATE_LABEL[status];
  if (!labelKey) return null;
  const declared = stateApplies(route, status);
  const isFailure = status === "error" || status === "offline" || status === "permission-denied";
  return (
    <div className="pill" role={isFailure ? "alert" : "status"} data-state={status}>
      <strong>{t(locale, labelKey)}</strong>
      {reasonCode ? (
        <span>
          {" "}
          · {t(locale, "reasonCodeLabel")}: <code>{reasonCode}</code>
        </span>
      ) : null}
      <p>
        {t(locale, "nextStepLabel")}: {t(locale, STATE_NEXT[status] ?? "nextError")}
      </p>
      {!declared ? (
        // The contract said this page could not reach this state. Surface the
        // contradiction rather than hiding it behind a generic message.
        <p data-undeclared="true">{t(locale, "stateNotApplicable")}</p>
      ) : null}
      {retryable ? (
        <button type="button" onClick={onRetry} title={t(locale, "retryNoWiden")}>
          {t(locale, "retry")}
        </button>
      ) : null}
    </div>
  );
}

/**
 * A page backed by one GET, rendered through the C04 state contract.
 *
 * The body is still a JSON dump: giving each page its own presentation is a
 * separate piece of work. What changed is that the page now says which state
 * it is in, why, and what to do next.
 */
function StateView({
  path,
  route,
  title,
  locale,
}: {
  path: string;
  route: string;
  title: string;
  locale: Locale;
}) {
  const res = useResource(path);
  return (
    <section className="panel">
      <h1>{title}</h1>
      {res.status === "loading" ? (
        <p role="status">
          {t(locale, "loading")}{" "}
          <button type="button" onClick={res.cancel}>
            {t(locale, "cancel")}
          </button>
        </p>
      ) : null}
      {res.status === "cancelled" ? (
        <p role="status">
          {t(locale, "cancelled")}{" "}
          <button type="button" onClick={res.retry}>
            {t(locale, "retry")}
          </button>
        </p>
      ) : null}
      <StateBanner
        route={route}
        status={res.status}
        reasonCode={res.reasonCode}
        retryable={res.retryable}
        locale={locale}
        onRetry={res.retry}
      />
      {res.data != null ? (
        <pre className="mono">{JSON.stringify(res.data, null, 2)}</pre>
      ) : null}
    </section>
  );
}

/**
 * A modal confirmation for a destructive action.
 *
 * Uses the native `<dialog>` so focus moves in, is trapped, and returns to
 * the opener on close, and Esc works — all without reimplementing it. Esc is
 * blocked only while the action is in flight, because dismissing then would
 * leave the user unsure whether it ran.
 */
function ConfirmDialog({
  open,
  busy,
  title,
  consequences,
  confirmLabel,
  locale,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  busy: boolean;
  title: string;
  consequences: string[];
  confirmLabel: string;
  locale: Locale;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLDialogElement | null>(null);

  useEffect(() => {
    const node = ref.current;
    if (!node) return;
    if (open && !node.open) node.showModal();
    if (!open && node.open) node.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      aria-busy={busy}
      onCancel={(event) => {
        // Esc during execution would hide an action that is still running.
        if (busy) event.preventDefault();
        else onCancel();
      }}
    >
      <h2>{title}</h2>
      <ul>
        {consequences.map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
      <div className="row">
        <button type="button" className="primary" disabled={busy} onClick={onConfirm}>
          {busy ? t(locale, "loading") : confirmLabel}
        </button>
        <button type="button" disabled={busy} onClick={onCancel}>
          {t(locale, "cancel")}
        </button>
      </div>
    </dialog>
  );
}

/**
 * V04 Receipt detail with its two actions.
 *
 * Verify re-derives the MAC from the stored record. Delete is destructive and
 * therefore states its consequences before it runs: the Receipt becomes a
 * tombstone, derived analyses stop resolving, and copies already exported
 * cannot be recalled.
 */
function ReceiptDetail({ locale }: { locale: Locale }) {
  const { id } = useParams();
  const path = `/api/v1/receipts/${id ?? ""}`;
  const res = useResource(path);
  const [verify, setVerify] = useState<Json | null>(null);
  const [verifyProblem, setVerifyProblem] = useState<{ code: string; message: string } | null>(
    null,
  );
  const [busy, setBusy] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [deleted, setDeleted] = useState<Json | null>(null);
  const [actionProblem, setActionProblem] = useState<{ code: string; message: string } | null>(
    null,
  );

  async function runVerify() {
    if (busy) return;
    setBusy("verify");
    setVerify(null);
    setVerifyProblem(null);
    const result = await requestJson(`${path}/verify`, { method: "POST", body: "{}" });
    if (result.ok) setVerify(asObj(result.data));
    else setVerifyProblem({ code: result.code, message: result.message });
    setBusy("");
  }

  async function runDelete() {
    if (busy) return;
    setBusy("delete");
    setActionProblem(null);
    const result = await requestJson(`${path}/delete`, { method: "POST", body: "{}" });
    if (result.ok) {
      setDeleted(asObj(result.data));
      setConfirming(false);
    } else {
      setActionProblem({ code: result.code, message: result.message });
      setConfirming(false);
    }
    setBusy("");
  }

  return (
    <section className="panel">
      <h1>
        {t(locale, "receipts")} {id}
      </h1>
      {res.status === "loading" ? (
        <p role="status">
          {t(locale, "loading")}{" "}
          <button type="button" onClick={res.cancel}>
            {t(locale, "cancel")}
          </button>
        </p>
      ) : null}
      <StateBanner
        route="/receipts"
        status={res.status}
        reasonCode={res.reasonCode}
        retryable={res.retryable}
        locale={locale}
        onRetry={res.retry}
      />

      <div className="row">
        <button type="button" disabled={busy !== "" || deleted !== null} onClick={() => void runVerify()}>
          {busy === "verify" ? t(locale, "loading") : t(locale, "receiptVerify")}
        </button>
        <button
          type="button"
          disabled={busy !== "" || deleted !== null}
          onClick={() => setConfirming(true)}
        >
          {t(locale, "receiptDelete")}
        </button>
      </div>

      {verify ? (
        <p role="status" data-testid="verify-result">
          {t(locale, "receiptVerified")} ·{" "}
          {/* A local MAC is not an organization signature, and says so. */}
          org_identity: {String(verify.org_identity)}
        </p>
      ) : null}
      {verifyProblem ? (
        <p role="alert" data-testid="verify-error">
          {t(locale, "reasonCodeLabel")}: <code>{verifyProblem.code}</code>
          {verifyProblem.message ? ` — ${verifyProblem.message}` : null}
        </p>
      ) : null}
      {actionProblem ? (
        <p role="alert" data-testid="delete-error">
          {t(locale, "reasonCodeLabel")}: <code>{actionProblem.code}</code>
          {actionProblem.message ? ` — ${actionProblem.message}` : null}
        </p>
      ) : null}
      {deleted ? (
        <div role="status" data-testid="delete-result">
          <p>{t(locale, "receiptDeleted")}</p>
          <pre className="mono">{JSON.stringify(deleted, null, 2)}</pre>
        </div>
      ) : null}

      <ConfirmDialog
        open={confirming}
        busy={busy === "delete"}
        title={t(locale, "receiptDeleteTitle")}
        consequences={[
          t(locale, "receiptDeleteTombstone"),
          t(locale, "receiptDeleteDerived"),
          t(locale, "receiptDeleteExternal"),
        ]}
        confirmLabel={t(locale, "receiptDelete")}
        locale={locale}
        onConfirm={() => void runDelete()}
        onCancel={() => setConfirming(false)}
      />

      {res.data != null && deleted === null ? (
        <pre className="mono">{JSON.stringify(res.data, null, 2)}</pre>
      ) : null}
    </section>
  );
}

function EntityPage({ folder, locale }: { folder: string; locale: Locale }) {
  const { id } = useParams();
  if (!id) {
    return (
      <section className="panel">
        <h1>{t(locale, folder)}</h1>
        <p role="alert">{t(locale, "missingId")}</p>
      </section>
    );
  }
  return (
    <StateView
      path={`/api/v1/${folder}/${id}`}
      route={`/${folder}`}
      title={`${t(locale, folder)} / ${id}`}
      locale={locale}
    />
  );
}

/** One editor control, rendered from the field's published spec. */
function SettingsField({
  path,
  spec,
  value,
  locale,
  disabled,
  onChange,
}: {
  path: string;
  spec: Json;
  value: unknown;
  locale: Locale;
  disabled: boolean;
  onChange: (path: string, value: unknown) => void;
}) {
  const kind = String(spec.kind ?? "");
  if (kind === "object") {
    const fields = asObj(spec.fields);
    const nested = asObj(value);
    return (
      <fieldset>
        <legend>{path}</legend>
        {Object.entries(fields).map(([name, childSpec]) => (
          <SettingsField
            key={`${path}.${name}`}
            path={`${path}.${name}`}
            spec={asObj(childSpec)}
            value={nested[name]}
            locale={locale}
            disabled={disabled}
            onChange={onChange}
          />
        ))}
      </fieldset>
    );
  }
  if (kind === "const_bool") {
    // An invariant, not a preference: shown so it is visible, and not
    // editable because the store refuses to change it.
    return (
      <label className="row">
        <span>{path}</span>
        <input type="checkbox" checked={value === true} disabled readOnly />
        <span className="muted">{t(locale, "settingsInvariant")}</span>
      </label>
    );
  }
  if (kind === "bool") {
    return (
      <label className="row">
        <span>{path}</span>
        <input
          type="checkbox"
          checked={value === true}
          disabled={disabled}
          onChange={(e) => onChange(path, e.target.checked)}
        />
      </label>
    );
  }
  if (kind === "enum") {
    const values = Array.isArray(spec.values) ? (spec.values as unknown[]) : [];
    return (
      <label className="row">
        <span>{path}</span>
        <select
          value={String(value ?? "")}
          disabled={disabled}
          onChange={(e) => onChange(path, e.target.value)}
        >
          {values.map((item) => (
            <option key={String(item)} value={String(item)}>
              {String(item)}
            </option>
          ))}
        </select>
      </label>
    );
  }
  if (kind === "int") {
    return (
      <label className="row">
        <span>
          {path} <span className="muted">[{String(spec.min)}, {String(spec.max)}]</span>
        </span>
        <input
          type="number"
          value={typeof value === "number" ? value : ""}
          min={typeof spec.min === "number" ? spec.min : undefined}
          max={typeof spec.max === "number" ? spec.max : undefined}
          disabled={disabled}
          onChange={(e) => {
            const raw = e.target.value;
            // Keep an empty box distinguishable from 0 so the draft does not
            // silently become a valid-looking value while being edited.
            onChange(path, raw === "" ? raw : Number(raw));
          }}
        />
      </label>
    );
  }
  return (
    <p role="alert">
      {path}: {t(locale, "settingsSpecUnknown")}
    </p>
  );
}

/**
 * V12 Settings: edit, validate, save, revert.
 *
 * The editor is generated from `/api/v1/settings/schema`, so it cannot offer
 * a field or a value the store does not accept. Client-side validation is a
 * convenience; the save still goes through the store's own validation, and a
 * refusal is shown with the store's reason code.
 */
/**
 * V05 Assets: vet, preview, copy, roll back.
 *
 * An asset id names a registry entry. The registry — version-controlled in
 * the project — decides where bytes come from and where they land, so this
 * page never chooses a filesystem path. Copy is offered only after a preview
 * has vetted the bytes, and the preview is where an unlicensed, unregistered
 * or tampered asset is refused.
 */
function AssetsPage({ locale }: { locale: Locale }) {
  const res = useResource("/api/v1/assets");
  const data = asObj(res.data);
  const [assetId, setAssetId] = useState("");
  const [plan, setPlan] = useState<Json | null>(null);
  const [copied, setCopied] = useState<Json | null>(null);
  const [busy, setBusy] = useState("");
  const [problem, setProblem] = useState<{ code: string; message: string } | null>(null);

  async function call(action: "preview" | "copy") {
    if (busy || !assetId) return;
    setBusy(action);
    setProblem(null);
    const result = await requestJson(`/api/v1/assets/${encodeURIComponent(assetId)}/${action}`, {
      method: "POST",
      body: "{}",
    });
    if (result.ok) {
      const value = asObj(result.data);
      if (action === "preview") {
        setPlan(value);
        setCopied(null);
      } else {
        setCopied(value);
        res.retry();
      }
    } else {
      setProblem({ code: result.code, message: result.message });
      if (action === "preview") setPlan(null);
    }
    setBusy("");
  }

  async function rollback() {
    const tx = String(asObj(copied).tx_id ?? "");
    if (busy || !tx) return;
    setBusy("rollback");
    setProblem(null);
    const result = await requestJson(`/api/v1/assets/${encodeURIComponent(tx)}/rollback`, {
      method: "POST",
      body: "{}",
    });
    if (result.ok) {
      setCopied(null);
      setPlan(null);
      res.retry();
    } else {
      setProblem({ code: result.code, message: result.message });
    }
    setBusy("");
  }

  const sbom = asObj(data.sbom);
  const unlicensed = typeof sbom.unlicensed_components === "number" ? sbom.unlicensed_components : 0;

  return (
    <section className="panel">
      <h1>{t(locale, "assets")}</h1>
      <StateBanner
        route="/assets"
        status={res.status}
        reasonCode={res.reasonCode}
        retryable={res.retryable}
        locale={locale}
        onRetry={res.retry}
      />
      <p className="muted">{t(locale, "assetsApmAuthority")}</p>
      <p className="muted">{String(data.copy_executor_scope ?? "")}</p>

      <div className="row">
        <label>
          asset_id
          <input
            value={assetId}
            disabled={busy !== ""}
            onChange={(e) => setAssetId(e.target.value)}
            placeholder={t(locale, "assetsIdPlaceholder")}
          />
        </label>
        <button type="button" disabled={busy !== "" || !assetId} onClick={() => void call("preview")}>
          {busy === "preview" ? t(locale, "loading") : t(locale, "assetsPreview")}
        </button>
        <button
          type="button"
          className="primary"
          // Copy follows a vetted preview; there is no blind install.
          disabled={busy !== "" || plan === null || copied !== null}
          onClick={() => void call("copy")}
          title={plan === null ? t(locale, "assetsPreviewFirst") : undefined}
        >
          {busy === "copy" ? t(locale, "loading") : t(locale, "assetsCopy")}
        </button>
        {copied ? (
          <button type="button" disabled={busy !== ""} onClick={() => void rollback()}>
            {busy === "rollback" ? t(locale, "loading") : t(locale, "assetsRollback")}
          </button>
        ) : null}
      </div>

      {plan === null && !problem ? <p className="muted">{t(locale, "assetsPreviewFirst")}</p> : null}
      {problem ? (
        <p role="alert" data-testid="assets-error">
          {t(locale, "reasonCodeLabel")}: <code>{problem.code}</code>
          {problem.message ? ` — ${problem.message}` : null}
        </p>
      ) : null}

      {plan ? (
        <dl data-testid="assets-plan">
          <dt>{t(locale, "assetsLicense")}</dt>
          <dd>{String(plan.license ?? "-")}</dd>
          <dt>{t(locale, "assetsOrigin")}</dt>
          <dd>{String(plan.origin ?? "-")}</dd>
          <dt>{t(locale, "assetsTarget")}</dt>
          <dd>{String(plan.target_rel ?? "-")}</dd>
          <dt>{t(locale, "assetsLoss")}</dt>
          <dd>{String(plan.loss ?? "-")}</dd>
        </dl>
      ) : null}
      {copied ? (
        <p role="status" data-testid="assets-copied">
          {t(locale, "assetsCopied")} · tx <code>{String(copied.tx_id ?? "")}</code>
        </p>
      ) : null}

      {unlicensed > 0 ? (
        <p role="alert" data-testid="assets-unlicensed">
          {t(locale, "assetsUnlicensed")}: {unlicensed}
        </p>
      ) : null}

      {res.data != null ? (
        <pre className="mono">{JSON.stringify(res.data, null, 2)}</pre>
      ) : null}
    </section>
  );
}

/**
 * V08 Sync: preview, then apply, with transport and semantic kept apart.
 *
 * Apply is only offered after a clean preview, so the destructive step always
 * follows a stated outcome. Transport success is displayed next to — never
 * merged into — the semantic result, because moving bytes is not verifying
 * meaning.
 */
function SyncPage({ locale }: { locale: Locale }) {
  const status = useResource("/api/v1/sync");
  const [bundleId, setBundleId] = useState("bundle-local");
  const [preview, setPreview] = useState<Json | null>(null);
  const [applied, setApplied] = useState<Json | null>(null);
  const [busy, setBusy] = useState("");
  const [problem, setProblem] = useState<{ code: string; message: string } | null>(null);

  async function call(action: "preview" | "apply") {
    if (busy) return;
    setBusy(action);
    setProblem(null);
    const result = await requestJson(`/api/v1/sync/${action}`, {
      method: "POST",
      body: JSON.stringify({ bundle_id: bundleId }),
    });
    if (result.ok) {
      const value = asObj(result.data);
      if (action === "preview") {
        setPreview(value);
        setApplied(null);
      } else {
        setApplied(value);
      }
    } else {
      setProblem({ code: result.code, message: result.message });
      if (action === "preview") setPreview(null);
    }
    setBusy("");
  }

  const conflict = preview?.conflict === true;
  const outcome = applied ?? preview;

  return (
    <section className="panel">
      <h1>{t(locale, "sync")}</h1>
      <StateBanner
        route="/sync"
        status={status.status}
        reasonCode={status.reasonCode}
        retryable={status.retryable}
        locale={locale}
        onRetry={status.retry}
      />
      <p className="muted">{t(locale, "syncDestFixed")}</p>

      <div className="row">
        <label>
          bundle_id
          <input
            value={bundleId}
            disabled={busy !== ""}
            onChange={(e) => setBundleId(e.target.value)}
          />
        </label>
        <button type="button" disabled={busy !== ""} onClick={() => void call("preview")}>
          {busy === "preview" ? t(locale, "loading") : t(locale, "syncPreview")}
        </button>
        <button
          type="button"
          className="primary"
          // Apply follows a clean preview: never a blind write, and never
          // over a conflict.
          disabled={busy !== "" || preview === null || conflict || applied !== null}
          onClick={() => void call("apply")}
          title={preview === null ? t(locale, "syncPreviewFirst") : undefined}
        >
          {busy === "apply" ? t(locale, "loading") : t(locale, "syncApply")}
        </button>
      </div>

      {preview === null && !problem ? <p className="muted">{t(locale, "syncPreviewFirst")}</p> : null}
      {conflict ? (
        <p role="alert" data-testid="sync-conflict">
          {t(locale, "syncConflict")} · {t(locale, "reasonCodeLabel")}:{" "}
          <code>{String(preview?.reason_code ?? "sync.conflict")}</code>
        </p>
      ) : null}
      {problem ? (
        <p role="alert" data-testid="sync-error">
          {t(locale, "reasonCodeLabel")}: <code>{problem.code}</code>
          {problem.message ? ` — ${problem.message}` : null}
        </p>
      ) : null}

      {outcome ? (
        <dl data-testid="sync-outcome">
          <dt>{t(locale, "syncTransport")}</dt>
          <dd>
            {String(outcome.transport ?? "-")}
            {/* Stated on the same row it could be mistaken for. */}
            <span className="muted"> · {t(locale, "syncTransportNotVerified")}</span>
          </dd>
          <dt>{t(locale, "syncSemantic")}</dt>
          <dd>{String(outcome.semantic ?? "-")}</dd>
          {outcome.reconciliation ? (
            <>
              <dt>{t(locale, "syncReconciliation")}</dt>
              <dd>{String(outcome.reconciliation)}</dd>
            </>
          ) : null}
        </dl>
      ) : null}

      {status.data != null ? (
        <pre className="mono">{JSON.stringify(status.data, null, 2)}</pre>
      ) : null}
    </section>
  );
}

function SettingsPage({ locale }: { locale: Locale }) {
  const [schema, setSchema] = useState<Json>({});
  const [saved, setSaved] = useState<Json>({});
  const [draft, setDraft] = useState<Json>({});
  const [load, setLoad] = useState<StateVerdict | null>(null);
  const [saving, setSaving] = useState(false);
  const [saveProblem, setSaveProblem] = useState<{ code: string; message: string } | null>(null);
  const [savedAt, setSavedAt] = useState("");

  async function reload() {
    setLoad(null);
    const [schemaResult, valueResult] = await Promise.all([
      requestJson("/api/v1/settings/schema"),
      requestJson("/api/v1/settings"),
    ]);
    if (!schemaResult.ok) {
      setLoad(classifyFailure(schemaResult.kind, schemaResult.code));
      return;
    }
    if (!valueResult.ok) {
      setLoad(classifyFailure(valueResult.kind, valueResult.code));
      return;
    }
    const fields = asObj(asObj(schemaResult.data).fields);
    // Project onto the schema: the response envelope carries keys that are
    // not settings, and sending them back would be refused.
    const stored = projectToSchema(fields, valueResult.data) as Json;
    setSchema(fields);
    setSaved(stored);
    setDraft(stored);
    setLoad(classifyPayload(valueResult.data));
  }

  useEffect(() => {
    void reload();
  }, []);

  const problems = useMemo(() => validateDraft(schema, draft), [schema, draft]);
  const dirty = useMemo(() => changedFields(saved, draft), [saved, draft]);

  async function save() {
    if (saving || problems.length > 0 || dirty.length === 0) return;
    setSaving(true);
    setSaveProblem(null);
    const result = await requestJson("/api/v1/settings", {
      method: "POST",
      body: JSON.stringify(draft),
    });
    if (result.ok) {
      const stored = projectToSchema(schema, result.data) as Json;
      setSaved(stored);
      setDraft(stored);
      setSavedAt(new Date().toISOString());
    } else {
      // The store is the authority; show exactly what it refused.
      setSaveProblem({ code: result.code, message: result.message });
    }
    setSaving(false);
  }

  return (
    <section className="panel">
      <h1>{t(locale, "settings")}</h1>
      <p>{t(locale, "vaultDefault")}</p>
      <SharedStateBanner
        route="/settings"
        verdict={load}
        locale={locale}
        onRetry={() => void reload()}
      />
      {Object.keys(schema).length === 0 ? null : (
        <>
          <div className="settings-form">
            {Object.entries(schema).map(([name, spec]) => (
              <SettingsField
                key={name}
                path={name}
                spec={asObj(spec)}
                value={draft[name]}
                locale={locale}
                disabled={saving}
                onChange={(path, value) => setDraft((current) => setPath(current, path, value))}
              />
            ))}
          </div>

          {problems.length > 0 ? (
            <ul role="alert" data-testid="settings-problems">
              {problems.map((problem) => (
                <li key={problem.path}>
                  <code>{problem.path}</code> · <code>{problem.code}</code> — {problem.message}
                </li>
              ))}
            </ul>
          ) : null}

          {saveProblem ? (
            <p role="alert" data-testid="settings-save-error">
              {t(locale, "reasonCodeLabel")}: <code>{saveProblem.code}</code>
              {saveProblem.message ? ` — ${saveProblem.message}` : null}
            </p>
          ) : null}

          <p aria-live="polite">
            {dirty.length > 0
              ? `${t(locale, "settingsUnsaved")}: ${dirty.join(", ")}`
              : savedAt
                ? t(locale, "settingsSaved")
                : t(locale, "settingsNoChanges")}
          </p>

          <div className="row">
            <button
              type="button"
              className="primary"
              // Disabled while saving so a second click cannot submit twice.
              disabled={saving || dirty.length === 0 || problems.length > 0}
              onClick={() => void save()}
            >
              {saving ? t(locale, "loading") : t(locale, "settingsSave")}
            </button>
            <button
              type="button"
              disabled={saving || dirty.length === 0}
              onClick={() => setDraft(saved)}
            >
              {t(locale, "settingsRevert")}
            </button>
          </div>
        </>
      )}
    </section>
  );
}

function CarePlanPage({ locale }: { locale: Locale }) {
  const { findingId } = useParams();
  return (
    <section className="panel">
      <h1>{t(locale, "carePlan")}</h1>
      <p>
        {t(locale, "carePlanFinding")} {findingId}
      </p>
      <p>{t(locale, "carePlanLocked")}</p>
      <StateView
        path={`/api/v1/care-plan/${findingId ?? ""}`}
        route="/care-plan/:findingId"
        title={t(locale, "plan")}
        locale={locale}
      />
    </section>
  );
}

function IntegrationsPage({ locale }: { locale: Locale }) {
  return (
    <section className="panel">
      <h1>{t(locale, "integrations")}</h1>
      <p>{t(locale, "integrationsIndependence")}</p>
      <AdapterCoverage locale={locale} />
      <StateView
        path="/api/v1/integrations"
        route="/integrations"
        title={t(locale, "catalog")}
        locale={locale}
      />
    </section>
  );
}
