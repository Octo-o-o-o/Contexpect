import { NavLink, Navigate, Route, Routes, useLocation, useParams } from "react-router-dom";
import { useEffect, useMemo, useState, type ClipboardEvent, type KeyboardEvent } from "react";
import { NAV } from "./routes";
import { t, type Locale } from "./i18n";
import { asObj, getJson, postJson, type Json } from "./api";
import { projectFieldValue, projectVisible, revealed } from "./mask";

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
      return;
    }
    setStatus("loading");
    setError("");
    try {
      const out = asObj(
        await postJson("/api/v1/inspect", { project, harness: "codex", symptom: symptom ?? "" }),
      );
      setReceipt(asObj(out.receipt));
      if (out.stale === true) setStatus("stale");
      else setStatus("idle");
      const query = symptom ? `?symptom=${encodeURIComponent(symptom)}` : "";
      const doc = asObj(await getJson(`/api/v1/doctor${query}`));
      setDoctor(doc);
    } catch (err) {
      setStatus("error");
      setError(String(err));
    }
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
                  onDiagnose={(symptom) => void runInspect(symptom)}
                  onCopy={confirmCopy}
                />
              }
            />
            <Route path="/checkup" element={<CheckupPage receipt={receipt} status={status} error={error} locale={locale} />} />
            <Route
              path="/inspector"
              element={
                <InspectorPage
                  receipt={receipt}
                  hold={isRevealed}
                  locale={locale}
                  onCopy={confirmCopy}
                />
              }
            />
            <Route path="/compare" element={<ComparePage locale={locale} />} />
            <Route path="/receipts" element={<SimpleGet path="/api/v1/receipts" title={t(locale, "receipts")} locale={locale} />} />
            <Route path="/receipts/:id" element={<ReceiptDetail locale={locale} />} />
            <Route path="/assets" element={<SimpleGet path="/api/v1/assets" title={t(locale, "assets")} locale={locale} />} />
            <Route path="/assets/:id" element={<EntityPage folder="assets" locale={locale} />} />
            <Route path="/sessions" element={<SimpleGet path="/api/v1/sessions" title={t(locale, "sessions")} locale={locale} />} />
            <Route path="/sessions/:id" element={<EntityPage folder="sessions" locale={locale} />} />
            <Route path="/monitor" element={<SimpleGet path="/api/v1/monitor" title={t(locale, "monitor")} locale={locale} />} />
            <Route path="/lab" element={<SimpleGet path="/api/v1/lab" title={t(locale, "lab")} locale={locale} />} />
            <Route path="/lab/:id" element={<EntityPage folder="lab" locale={locale} />} />
            <Route path="/sync" element={<SimpleGet path="/api/v1/sync" title={t(locale, "sync")} locale={locale} />} />
            <Route path="/policy" element={<SimpleGet path="/api/v1/policy" title={t(locale, "policy")} locale={locale} />} />
            <Route path="/standards" element={<SimpleGet path="/api/v1/standards" title={t(locale, "standards")} locale={locale} />} />
            <Route path="/standards/:id" element={<EntityPage folder="standards" locale={locale} />} />
            <Route path="/settings" element={<SettingsPage locale={locale} />} />
            <Route path="/exceptions" element={<SimpleGet path="/api/v1/exceptions" title={t(locale, "exceptions")} locale={locale} />} />
            <Route path="/team/compliance" element={<SimpleGet path="/api/v1/team/compliance" title={t(locale, "team")} locale={locale} />} />
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
                  onClick={() => setSelected(index)}
                  onKeyDown={(e) => e.key === "Enter" && setSelected(index)}
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
        <AdapterCoverage />
      </section>
      <aside className="panel drawer">
        <h2>{t(locale, "diagnosisEvidence")}</h2>
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

function AdapterCoverage() {
  const [data, setData] = useState<Json>({});
  useEffect(() => {
    void getJson("/api/v1/integrations")
      .then((v) => setData(asObj(v)))
      .catch(() => setData({}));
  }, []);
  const families = Array.isArray(data.families) ? (data.families as Json[]) : [];
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
  locale,
}: {
  receipt: Json;
  status: string;
  error: string;
  locale: Locale;
}) {
  return (
    <section className="panel">
      <h1>{t(locale, "checkup")}</h1>
      <p>{t(locale, "checkupLead")}</p>
      <p>
        {t(locale, "statusLabel")}: {status}
      </p>
      {error ? <p role="alert">{error}</p> : null}
      <pre className="mono">{JSON.stringify(receipt.policy_result ?? {}, null, 2)}</pre>
    </section>
  );
}

function InspectorPage({
  receipt,
  hold,
  locale,
  onCopy,
}: {
  receipt: Json;
  hold: boolean;
  locale: Locale;
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
  useEffect(() => {
    void getJson("/api/v1/receipts")
      .then((v) => {
        const list = asObj(v).receipts;
        setReceipts(Array.isArray(list) ? (list as Json[]) : []);
      })
      .catch((e: unknown) => setErr(String(e)));
  }, []);
  async function runDiff() {
    if (!a || !b) return;
    setLoading(true);
    setErr("");
    try {
      const out = await getJson(`/api/v1/diff?a=${encodeURIComponent(a)}&b=${encodeURIComponent(b)}`);
      setDiff(out);
    } catch (e: unknown) {
      setErr(String(e));
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

function SimpleGet({ path, title, locale }: { path: string; title: string; locale: Locale }) {
  const [data, setData] = useState<unknown>(null);
  const [err, setErr] = useState("");
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    setLoading(true);
    void getJson(path)
      .then((v) => {
        setData(v);
        setLoading(false);
      })
      .catch((e: unknown) => {
        setErr(String(e));
        setLoading(false);
      });
  }, [path]);
  return (
    <section className="panel">
      <h1>{title}</h1>
      {loading ? <p>{t(locale, "loading")}</p> : null}
      {err ? <p role="alert">{err}</p> : null}
      {!loading && !err && data == null ? <p>{t(locale, "empty")}</p> : null}
      {!err && data != null ? <pre className="mono">{JSON.stringify(data, null, 2)}</pre> : null}
    </section>
  );
}

function ReceiptDetail({ locale }: { locale: Locale }) {
  const { id } = useParams();
  return <SimpleGet path={`/api/v1/receipts/${id ?? ""}`} title={`${t(locale, "receipts")} ${id}`} locale={locale} />;
}

function EntityPage({ folder, locale }: { folder: string; locale: Locale }) {
  const { id } = useParams();
  const [data, setData] = useState<unknown>(null);
  const [err, setErr] = useState("");
  const [loading, setLoading] = useState(true);
  useEffect(() => {
    if (!id) {
      setErr(t(locale, "missingId"));
      setLoading(false);
      return;
    }
    setLoading(true);
    void getJson(`/api/v1/${folder}/${id}`)
      .then((v) => {
        setData(v);
        setLoading(false);
      })
      .catch((e: unknown) => {
        setErr(String(e));
        setLoading(false);
      });
  }, [folder, id, locale]);
  return (
    <section className="panel">
      <h1>
        {t(locale, folder)} / {id}
      </h1>
      {loading ? <p>{t(locale, "loading")}</p> : null}
      {err ? <p role="alert">{err}</p> : null}
      {!loading && !err && (data == null || (typeof data === "object" && Object.keys(asObj(data)).length === 0)) ? (
        <p>{t(locale, "empty")}</p>
      ) : null}
      {!err && data != null ? <pre className="mono">{JSON.stringify(data, null, 2)}</pre> : null}
    </section>
  );
}

function SettingsPage({ locale }: { locale: Locale }) {
  const [data, setData] = useState<Json>({});
  const [err, setErr] = useState("");
  useEffect(() => {
    void getJson("/api/v1/settings")
      .then((v) => setData(asObj(v)))
      .catch((e: unknown) => setErr(String(e)));
  }, []);
  return (
    <section className="panel">
      <h1>{t(locale, "settings")}</h1>
      <p>{t(locale, "vaultDefault")}</p>
      {err ? <p role="alert">{err}</p> : null}
      <pre className="mono">{JSON.stringify(data, null, 2)}</pre>
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
      <SimpleGet path={`/api/v1/care-plan/${findingId ?? ""}`} title={t(locale, "plan")} locale={locale} />
    </section>
  );
}

function IntegrationsPage({ locale }: { locale: Locale }) {
  return (
    <section className="panel">
      <h1>{t(locale, "integrations")}</h1>
      <p>{t(locale, "integrationsIndependence")}</p>
      <AdapterCoverage />
      <SimpleGet path="/api/v1/integrations" title={t(locale, "catalog")} locale={locale} />
    </section>
  );
}
