import { NavLink, Navigate, Route, Routes, useLocation, useParams } from "react-router-dom";
import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type KeyboardEvent } from "react";
import { NAV } from "./routes";
import { t, type Locale } from "./i18n";
import { asObj, postJson, requestJson, type Json } from "./api";
import { projectFieldValue, projectVisible, revealed } from "./mask";
import { classifyFailure, classifyPayload } from "./page-state";
import { stateApplies } from "./page-contract";

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
          <NavLink key={item.to} to={item.to} className={({ isActive }) => (isActive ? "active" : "")}>
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
            <Route path="/assets" element={<StateView path="/api/v1/assets" route="/assets" title={t(locale, "assets")} locale={locale} />} />
            <Route path="/assets/:id" element={<EntityPage folder="assets" locale={locale} />} />
            <Route path="/sessions" element={<StateView path="/api/v1/sessions" route="/sessions" title={t(locale, "sessions")} locale={locale} />} />
            <Route path="/sessions/:id" element={<EntityPage folder="sessions" locale={locale} />} />
            <Route path="/monitor" element={<StateView path="/api/v1/monitor" route="/monitor" title={t(locale, "monitor")} locale={locale} />} />
            <Route path="/lab" element={<StateView path="/api/v1/lab" route="/lab" title={t(locale, "lab")} locale={locale} />} />
            <Route path="/lab/:id" element={<EntityPage folder="lab" locale={locale} />} />
            <Route path="/sync" element={<StateView path="/api/v1/sync" route="/sync" title={t(locale, "sync")} locale={locale} />} />
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

function ReceiptDetail({ locale }: { locale: Locale }) {
  const { id } = useParams();
  return (
    <StateView
      path={`/api/v1/receipts/${id ?? ""}`}
      route="/receipts"
      title={`${t(locale, "receipts")} ${id}`}
      locale={locale}
    />
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

function SettingsPage({ locale }: { locale: Locale }) {
  return (
    <>
      <p className="panel">{t(locale, "vaultDefault")}</p>
      <StateView
        path="/api/v1/settings"
        route="/settings"
        title={t(locale, "settings")}
        locale={locale}
      />
    </>
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
