import { NavLink, Navigate, Route, Routes, useLocation, useNavigate, useParams } from "react-router-dom";
import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type KeyboardEvent, type ReactNode, type RefObject } from "react";
import { createGeneration } from "./generation";
import { NAV, navKeyForPath } from "./routes";
import { t, type Locale } from "./i18n";
import { asObj, postJson, requestJson, type Json } from "./api";
import { projectFieldValue, projectVisible, revealed } from "./mask";
import { classifyFailure, classifyPayload } from "./page-state";
import { stateApplies } from "./page-contract";
import {
  changedFields,
  projectToSchema,
  setPath,
  validateDraft,
  type SettingsProblem,
} from "./settings-form";

const FACETS = [
  "installed",
  "discoverable",
  "eligible",
  "model-visible",
  "use-evidence",
  "outcome-affecting",
] as const;

/** Sidebar grouping of NAV; every NAV entry appears in exactly one group. */
const NAV_GROUPS: { key: string; items: string[] }[] = [
  { key: "navGroupDiagnose", items: ["doctor", "checkup", "inspector", "compare"] },
  { key: "navGroupRecords", items: ["receipts", "sessions", "monitor", "assets"] },
  { key: "navGroupActions", items: ["lab", "sync", "integrations"] },
  { key: "navGroupGovernance", items: ["policy", "standards", "exceptions", "team", "settings"] },
];

/** truth_state → the CSS class that colours it; unlisted values keep the default text colour. */
function truthClass(truth: string): string {
  if (truth === "present") return "t-present";
  if (truth === "absent") return "t-absent";
  if (truth === "indeterminate") return "t-indeterminate";
  if (truth === "not-applicable") return "t-na";
  return "";
}

export function App() {
  const [locale, setLocale] = useState<Locale>("zh-CN");
  const [privacy, setPrivacy] = useState<"default" | "screenshot">("default");
  const [project, setProject] = useState("");
  const [receipt, setReceipt] = useState<Json>({});
  const [doctor, setDoctor] = useState<Json>({});
  const [bootstrap, setBootstrap] = useState<Json>({});
  const [status, setStatus] = useState<"idle" | "loading" | "error" | "stale">("idle");
  const [error, setError] = useState("");
  // The C04 verdict for the shared inspect/doctor request, so Checkup,
  // Inspector and Doctor render the same banner as every other page instead
  // of a stringified exception.
  const [verdict, setVerdict] = useState<StateVerdict | null>(null);
  const [lastSymptom, setLastSymptom] = useState<string | undefined>(undefined);
  const [hold, setHold] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const paletteButtonRef = useRef<HTMLButtonElement | null>(null);
  const narrow = useNarrowViewport();
  const loc = useLocation();

  useEffect(() => {
    const onKey = (event: globalThis.KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    // Route changes are page changes here. Leaving the title fixed makes
    // every route read as the same page to assistive tech, to browser
    // history and to tab strips (WCAG 2.2 SC 2.4.2).
    const key = navKeyForPath(loc.pathname);
    const page = key ? t(locale, key) : "";
    document.title = page ? `${page} · Contexpect` : "Contexpect";
  }, [loc.pathname, locale]);
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

  // C01/C40: every inspect run gets a generation. A run that finishes after a
  // newer one started is dropped, so an older coordinate's answer can never
  // overwrite the current one.
  // The same helper `generation.test.mjs` exercises, not a second copy of
  // the pattern; the previous run's requests are also aborted, not only
  // ignored.
  const inspectGeneration = useRef(createGeneration());
  const inspectAbort = useRef<AbortController | null>(null);

  useEffect(() => {
    const ctrl = new AbortController();
    inspectAbort.current = ctrl;
    const generation = inspectGeneration.current.next();
    const current = () => !ctrl.signal.aborted && inspectGeneration.current.isCurrent(generation);
    void (async () => {
      setStatus("loading");
      const result = await requestJson("/api/v1/status", { signal: ctrl.signal });
      if (!current()) return;
      if (!result.ok) {
        setStatus("error");
        setError(result.code);
        setVerdict(classifyFailure(result.kind, result.code));
        return;
      }
      const data = asObj(result.data);
      setBootstrap(data);
      const id = asObj(data.selected_receipt).receipt_id;
      if (typeof id !== "string") {
        setStatus("idle");
        setVerdict({ state: "empty", reasonCode: "api.no_matching_receipt", retryable: true });
        return;
      }
      const [saved, diagnosed] = await Promise.all([
        requestJson(`/api/v1/receipts/${encodeURIComponent(id)}`, { signal: ctrl.signal }),
        requestJson(`/api/v1/doctor?receipt_id=${encodeURIComponent(id)}`, { signal: ctrl.signal }),
      ]);
      if (!current()) return;
      const failure = !saved.ok ? saved : !diagnosed.ok ? diagnosed : null;
      if (failure) {
        setStatus("error");
        setError(failure.code);
        setVerdict(classifyFailure(failure.kind, failure.code));
      } else if (saved.ok && diagnosed.ok) {
        const doc = asObj(diagnosed.data);
        if (asObj(saved.data).receipt_id !== id || doc.receipt_id !== id) {
          setStatus("error");
          setError("api.receipt_mismatch");
          setVerdict({ state: "error", reasonCode: "api.receipt_mismatch", retryable: true });
          return;
        }
        setReceipt(asObj(saved.data));
        setDoctor(doc);
        setStatus(asObj(data.staleness).status === "stale" ? "stale" : "idle");
        setVerdict(classifyPayload({ ...doc, staleness: data.staleness }));
      }
    })();
    return () => ctrl.abort();
  }, []);

  async function runInspect(symptom?: string) {
    if (!project && bootstrap.project !== "<project>") {
      setError(t(locale, "projectRequired"));
      setStatus("error");
      setVerdict({ state: "error", reasonCode: "ui.project_required", retryable: false });
      return;
    }
    inspectAbort.current?.abort();
    const ctrl = new AbortController();
    inspectAbort.current = ctrl;
    const generation = inspectGeneration.current.next();
    const current = () => inspectGeneration.current.isCurrent(generation);
    setStatus("loading");
    setBootstrap((previous) => ({ ...previous, staleness: { status: "unknown", reason_code: "ui.inspect_in_progress" } }));
    setError("");
    setVerdict(null);
    setLastSymptom(symptom);

    const inspected = await requestJson("/api/v1/inspect", {
      method: "POST",
      body: JSON.stringify({ ...(project ? { project } : {}), symptom: symptom ?? "" }),
      signal: ctrl.signal,
    });
    if (!current()) return;
    if (!inspected.ok) {
      // The reason code survives instead of being folded into a message.
      const failed = classifyFailure(inspected.kind, inspected.code);
      setStatus("error");
      setError(inspected.message || inspected.code);
      setVerdict({ ...failed, state: failed.state });
      return;
    }
    const out = asObj(inspected.data);
    const receiptObj = asObj(out.receipt);
    setReceipt(receiptObj);
    setDoctor({});
    setStatus(out.stale === true ? "stale" : "idle");

    // The diagnosis is asked for *this* Receipt by id. Without the id the
    // daemon would answer for whichever Receipt is current, and a later
    // inspect could make that a different observation than the one shown.
    const receiptId = typeof receiptObj.receipt_id === "string" ? receiptObj.receipt_id : "";
    const params = new URLSearchParams();
    if (receiptId) params.set("receipt_id", receiptId);
    if (symptom) params.set("symptom", symptom);
    const query = params.toString() ? `?${params.toString()}` : "";
    const diagnosed = await requestJson(`/api/v1/doctor${query}`, { signal: ctrl.signal });
    if (!current()) return;
    if (!diagnosed.ok) {
      const failed = classifyFailure(diagnosed.kind, diagnosed.code);
      setStatus("error");
      setError(diagnosed.message || diagnosed.code);
      setVerdict({ ...failed, state: failed.state });
      return;
    }
    const doc = asObj(diagnosed.data);
    if (doc.receipt_id !== receiptId) {
      setStatus("error");
      setError("api.receipt_mismatch");
      setVerdict({ state: "error", reasonCode: "api.receipt_mismatch", retryable: true });
      return;
    }
    const monitored = await requestJson(`/api/v1/monitor?receipt_id=${encodeURIComponent(receiptId)}`, { signal: ctrl.signal });
    if (!current()) return;
    const staleness = out.stale === true
      ? { status: "stale", reason_code: "ui.receipt_superseded" }
      : monitored.ok ? asObj(asObj(monitored.data).staleness)
        : { status: "unknown", reason_code: monitored.code };
    setBootstrap((previous) => ({ ...previous, selected_receipt: receiptObj, selection: "session-current", staleness }));
    setStatus(staleness.status === "stale" ? "stale" : "idle");
    setDoctor(doc);
    // Classify the diagnosis itself: Unknown cells make it partial, and a
    // stale Receipt stays stale.
    const payload = classifyPayload({ ...doc, staleness });
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

  const receiptQuery = typeof receipt.receipt_id === "string"
    ? `?receipt_id=${encodeURIComponent(receipt.receipt_id)}` : "";

  if (narrow) {
    // C06: below 768px the product is a read-only Receipt/notification
    // surface, not the full application with its navigation removed.
    return (
      <NarrowReadOnly
        locale={locale}
        privacy={privacy}
        hold={isRevealed}
        onCopy={confirmCopy}
      />
    );
  }

  return (
    <div className="shell" data-privacy={privacy} lang={locale}>
      {/* SC 2.4.1: sixteen nav links precede the content on every page.
          Without this, reaching the content by keyboard means tabbing past
          all of them, every time. */}
      <a className="skip-link" href="#main-content">
        {t(locale, "skipToContent")}
      </a>
      <nav className="nav" aria-label="primary">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">Cx</div>
          <div>
            <div className="brand-name">Contexpect</div>
            <div className="brand-sub">{t(locale, "brandSub")}</div>
          </div>
        </div>
        {NAV_GROUPS.map((group) => (
          <div className="nav-group" key={group.key}>
            <div className="nav-group-label">{t(locale, group.key)}</div>
            {NAV.filter((item) => group.items.includes(item.key)).map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                className={({ isActive }) => (isActive ? "active" : "")}
                // Below 1024px the label is collapsed visually. An explicit name
                // keeps the link identifiable to assistive tech regardless of how
                // a given AT treats visually-hidden text (WCAG 2.2 SC 2.4.4, 4.1.2).
                aria-label={t(locale, item.key)}
                title={t(locale, item.key)}
              >
                <span className="nav-dot" aria-hidden="true" />
                <span>{t(locale, item.key)}</span>
              </NavLink>
            ))}
          </div>
        ))}
        <div className="shell-foot">
          <div className="daemon">
            {String(asObj(bootstrap.daemon).listen ?? "—")}
          </div>
          <div>{t(locale, "daemonAddress")}</div>
        </div>
      </nav>
      {/* A real `main` landmark, so assistive tech can jump to the content
          rather than only walking the document. `tabIndex={-1}` lets the skip
          link move focus here without making it a tab stop. */}
      <main className="main" id="main-content" tabIndex={-1}>
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
          <span className="pill">
            {status === "loading"
              ? t(locale, "loading")
              : status === "stale"
                ? t(locale, "stale")
                : status === "error"
                  ? t(locale, "error")
                  : status === "idle"
                    ? t(locale, "statusReady")
                    : status}
          </span>
          <div className="topbar-right">
            <button
              type="button"
              className="cmdk-button"
              ref={paletteButtonRef}
              onClick={() => setPaletteOpen(true)}
              aria-label={t(locale, "cmdkOpen")}
            >
              {t(locale, "cmdkOpen")} <kbd>⌘K</kbd>
            </button>
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
        </div>
        <div className="coordinate-summary" data-testid="startup-coordinate">
          {String(bootstrap.project ?? "—")} · {String(asObj(receipt.coordinate).harness ?? asObj(bootstrap.coordinate).harness ?? "—")}
          {" · "}{String(receipt.receipt_id ?? "—")}
          {" · "}{t(locale, "snapshotOrigin")}: {String(bootstrap.selection ?? "—")}
          {" · "}{t(locale, "freshness")}: {String(asObj(bootstrap.staleness).status ?? "unknown")}
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
                  project={project || (bootstrap.project === "<project>" ? "<project>" : "")}
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
            <Route path="/receipts" element={<StateView path="/api/v1/receipts" route="/receipts" title={t(locale, "receipts")} locale={locale} render={(data) => <ReceiptsListView data={data} locale={locale} />} />} />
            <Route path="/receipts/:id" element={<ReceiptDetail locale={locale} />} />
            <Route path="/assets" element={<AssetsPage locale={locale} />} />
            <Route path="/assets/:id" element={<EntityPage folder="assets" locale={locale} />} />
            <Route path="/sessions" element={<StateView path="/api/v1/sessions" route="/sessions" title={t(locale, "sessions")} locale={locale} render={(data) => <SessionsListView data={data} locale={locale} />} />} />
            <Route path="/sessions/:id" element={<SessionRequestsPage locale={locale} />} />
            <Route path="/monitor" element={<StateView path={`/api/v1/monitor${receiptQuery}`} route="/monitor" title={t(locale, "monitor")} locale={locale} render={(data) => <MonitorView data={data} locale={locale} />} />} />
            <Route
              path="/lab"
              element={
                <StateView
                  path="/api/v1/lab"
                  route="/lab"
                  title={t(locale, "lab")}
                  locale={locale}
                  render={(data) => <LabListView data={data} locale={locale} />}
                />
              }
            />
            <Route
              path="/lab/:id"
              element={<EntityPage folder="lab" locale={locale} render={(data) => (
                <>
                  <LabResultView data={data} locale={locale} />
                  <RawJsonDetails data={data} locale={locale} />
                </>
              )} />}
            />
            <Route path="/sync" element={<SyncPage locale={locale} />} />
            <Route path="/policy" element={<StateView path="/api/v1/policy" route="/policy" title={t(locale, "policy")} locale={locale} render={(data) => <PolicyView data={data} locale={locale} />} />} />
            <Route path="/standards" element={<StateView path="/api/v1/standards" route="/standards" title={t(locale, "standards")} locale={locale} />} />
            <Route path="/standards/:id" element={<EntityPage folder="standards" locale={locale} />} />
            <Route path="/settings" element={<SettingsPage locale={locale} />} />
            <Route path="/exceptions" element={<StateView path="/api/v1/exceptions" route="/exceptions" title={t(locale, "exceptions")} locale={locale} render={(data) => <ExceptionsListView data={data} locale={locale} />} />} />
            <Route path="/team/compliance" element={<StateView path={`/api/v1/team/compliance${receiptQuery}`} route="/team/compliance" title={t(locale, "team")} locale={locale} render={(data) => <TeamComplianceView data={data} locale={locale} />} />} />
            <Route path="/care-plan/:findingId" element={<CarePlanPage locale={locale} receiptQuery={receiptQuery} />} />
            <Route path="/integrations" element={<IntegrationsPage locale={locale} />} />
            <Route path="/integrations/:id" element={<EntityPage folder="integrations" locale={locale} />} />
            <Route path="*" element={<p>{t(locale, "notFound")}: {loc.pathname}</p>} />
          </Routes>
        </div>
      </main>
      {paletteOpen ? (
        <CommandPalette
          locale={locale}
          onClose={() => setPaletteOpen(false)}
          triggerRef={paletteButtonRef}
        />
      ) : null}
    </div>
  );
}

/**
 * ⌘K / Ctrl+K jump-to-page palette.
 *
 * Mounted only while open, so SSR first paint never runs it; the global
 * shortcut listener lives in App's effect. Focus moves into the filter
 * input on open and back to the trigger button on close.
 */
function CommandPalette({
  locale,
  onClose,
  triggerRef,
}: {
  locale: Locale;
  onClose: () => void;
  triggerRef: RefObject<HTMLButtonElement | null>;
}) {
  const navigate = useNavigate();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement | null>(null);
  const dialogRef = useRef<HTMLDialogElement | null>(null);

  useEffect(() => {
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const dialog = dialogRef.current;
    dialog?.showModal();
    inputRef.current?.focus();
    return () => {
      dialog?.close();
      (previous?.isConnected ? previous : triggerRef.current)?.focus();
    };
  }, [triggerRef]);

  const needle = query.trim().toLowerCase();
  const items = NAV_GROUPS.flatMap((group) =>
    NAV.filter((item) => group.items.includes(item.key)).map((item) => ({
      to: item.to,
      key: item.key,
      label: t(locale, item.key),
      group: t(locale, group.key),
    })),
  ).filter((item) => needle === "" || item.label.toLowerCase().includes(needle));
  const current = items.length === 0 ? 0 : Math.min(active, items.length - 1);

  function pick(to: string) {
    onClose();
    navigate(to);
  }

  return (
    <dialog
      ref={dialogRef}
      className="palette"
      aria-label={t(locale, "cmdkOpen")}
      onCancel={(event) => { event.preventDefault(); onClose(); }}
      onKeyDown={(event) => {
        if (event.key !== "Tab") return;
        const controls = Array.from(event.currentTarget.querySelectorAll<HTMLElement>("input, button"));
        const first = controls[0];
        const last = controls[controls.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault(); last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault(); first?.focus();
        }
      }}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div>
        <input
          ref={inputRef}
          className="palette-input"
          value={query}
          placeholder={t(locale, "cmdkPlaceholder")}
          aria-label={t(locale, "cmdkPlaceholder")}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              onClose();
            } else if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((index) => Math.max(0, Math.min(index + 1, items.length - 1)));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((index) => Math.max(index - 1, 0));
            } else if (event.key === "Enter" && items[current]) {
              event.preventDefault();
              pick(items[current].to);
            }
          }}
        />
        <div className="palette-list">
          {items.length === 0 ? (
            <div className="palette-empty">{t(locale, "cmdkNoMatch")}</div>
          ) : (
            items.map((item, index) => (
              <button
                key={item.to}
                type="button"
                className={index === current ? "palette-item active" : "palette-item"}
                onMouseEnter={() => setActive(index)}
                onClick={() => pick(item.to)}
              >
                <span className="nav-dot" aria-hidden="true" />
                <span>{item.label}</span>
                <span className="palette-group">{item.group}</span>
              </button>
            ))
          )}
        </div>
      </div>
    </dialog>
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
        <div className="eyebrow">{t(locale, "navGroupDiagnose")}</div>
        <h1>{t(locale, "contextDoctor")}</h1>
        <div className="symptom-bar">
          <span className="symptom-prefix">{t(locale, "symptomPrefix")}</span>
          <input
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
        <div className="statband">
          <div className="stat c-confirmed">
            <div className="stat-lab">{t(locale, "confirmed")}</div>
            <div className="stat-num">{String(counts.confirmed ?? "—")}</div>
            <div className="stat-cap">{t(locale, "statCapConfirmed")}</div>
          </div>
          <div className="stat c-suspected">
            <div className="stat-lab">{t(locale, "suspected")}</div>
            <div className="stat-num">{String(counts.suspected ?? "—")}</div>
            <div className="stat-cap">{t(locale, "statCapSuspected")}</div>
          </div>
          <div className="stat c-unknown">
            <div className="stat-lab">{t(locale, "unknown")}</div>
            <div className="stat-num">{String(counts.unknown ?? "—")}</div>
            <div className="stat-cap">{t(locale, "statCapUnknown")}</div>
          </div>
        </div>
        <p className="muted">{String(counts.note ?? "")}</p>
        {typeof counts.active_confirmed === "number" ? <p data-testid="active-counts">
          {t(locale, "activeFindings")}: {String(counts.active_confirmed)} / {String(counts.active_blocking ?? "—")}
        </p> : null}
        {findings.length === 0 ? (
          <p>{t(locale, Object.keys(counts).length === 0 ? "diagnosisNotRun" : "emptyFindings")}</p>
        ) : (
          <table>
            {/* The table needs a name of its own; the surrounding heading is
                not attached to it. */}
            <caption className="sr-only">{t(locale, "findingsTableCaption")}</caption>
            <thead>
              <tr>
                <th>{t(locale, "findingSeverity")}</th>
                <th>{t(locale, "finding")}</th>
                <th>{t(locale, "affectedSurfaces")}</th>
                <th>{t(locale, "evidence")}</th>
                <th>{t(locale, "impact")}</th>
                <th>{t(locale, "firstSeen")}</th>
              </tr>
            </thead>
            <tbody>
              {findings.map((item, index) => {
                const severity = item.severity === "confirmed" || item.severity === "suspected"
                  ? item.severity : "";
                const surfaces = Array.isArray(item.affected_surfaces)
                  ? item.affected_surfaces
                  : [];
                return (
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
                  >
                    <td>
                      <span className={`sev ${severity}`}>
                        <span className="sev-mark" aria-hidden="true" />
                        {severity ? t(locale, severity) : "—"}
                      </span>
                    </td>
                    <td>
                      <div className="f-title">{String(item.title)}</div>
                      {item.suppressed === true ? <span className="pill">{t(locale, "suppressedFinding")}</span> : null}
                      <div className="f-meta">
                        <span className="mono">{String(item.finding_id)}</span>
                      </div>
                    </td>
                    <td>
                      {surfaces.length > 0
                        ? surfaces.map((surface) => (
                            <span key={String(surface)} className="agent-chip">
                              {String(surface)}
                            </span>
                          ))
                        : String(item.affected_surfaces ?? "")}
                    </td>
                    <td>
                      <EvidencePill
                        locale={locale}
                        kind={item.evidence_state === "indeterminate" ? "unknown" : "static"}
                      />
                    </td>
                    <td>{String(item.impact)}</td>
                    <td>
                      <span className="f-firstseen">
                        {item.first_seen === "current-receipt" ? t(locale, "firstSeenUntracked") : String(item.first_seen ?? "—")}
                      </span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
        <div className="sec-head">
          <h2>{t(locale, "adapterCoverage")}</h2>
        </div>
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
        {finding ? (
          <div className="rail-eyebrow">
            {t(locale, "diagnosisEvidence")} · <span className="mono">{String(finding.finding_id)}</span>
          </div>
        ) : null}
        <h2 id="drawer-title">{finding ? String(finding.title) : t(locale, "diagnosisEvidence")}</h2>
        {collectStatus === "loading" ? (
          <p role="status">{t(locale, "drawerBusy")}</p>
        ) : null}
        {finding ? (
          <>
            <div className={asObj(finding.treatment).locked === true ? "decision indeterminate" : "decision"}>
              <div className="d-lab">{t(locale, "decision")}</div>
              <div className="d-val">
                {asObj(finding.treatment).locked === true
                  ? t(locale, "decisionIndeterminate")
                  : t(locale, "decisionOpen")}
              </div>
              <div className="d-why">{String(finding.impact ?? "")}</div>
            </div>
            <EvidenceChain receipt={receipt} finding={finding} locale={locale} />
            <h2>{t(locale, "sixFacets")}</h2>
            <div className="facet-grid">
              {FACETS.map((name) => {
                const claim = asObj(facets[name]);
                const truth = String(claim.truth_state ?? "unknown");
                return (
                  <div key={name} className={truth === "indeterminate" ? "facet broken" : "facet"}>
                    <div className="f-lab">{name}</div>
                    <div className={truthClass(truth) ? `f-val ${truthClass(truth)}` : "f-val"}>{truth}</div>
                  </div>
                );
              })}
            </div>
            <div className="rail-actions">
              <button
                type="button"
                className="primary"
                disabled={!canAct || collectStatus === "loading"}
                onClick={() => void collectEvidence()}
                title={canAct ? undefined : t(locale, "collectDisabled")}
              >
                {collectStatus === "loading" ? t(locale, "loading") : t(locale, "collectEvidence")}
              </button>
              {!canAct ? <p className="muted">{t(locale, "collectDisabled")}</p> : null}
            </div>
            {collectError ? <p role="alert">{collectError}</p> : null}
            {collectResult.inventory_digest ? (
              <pre className="mono">{JSON.stringify(collectResult, null, 2)}</pre>
            ) : null}
            <div className="lockbox">
              <span className="lockicon" aria-hidden="true">⚿</span>
              <span>
                {t(locale, "treatmentLocked")}: {String(asObj(finding.treatment).locked === true)}. {t(locale, "cannotUnlock")}
              </span>
            </div>
            <MaskedText text={String(finding.reason_code)} hold={hold} locale={locale} onCopy={onCopy} />
            <p className="honest-note">{t(locale, "honestNoteStatic")}</p>
          </>
        ) : (
          <p className="muted">{t(locale, "selectFinding")}</p>
        )}
      </aside>
    </>
  );
}

function EvidenceChain({ receipt, finding, locale }: { receipt: Json; finding: Json; locale: Locale }) {
  const modelVisible = asObj(asObj(receipt.facets)["model-visible"]);
  return <dl className="kv" data-testid="evidence-facts">
    <dt>{t(locale, "evidence")}</dt><dd>{String(finding.evidence_state ?? "unknown")}</dd>
    <dt>{t(locale, "evidenceModelVisible")}</dt><dd>{String(modelVisible.truth_state ?? "unknown")}</dd>
    <dt>claim_kind</dt><dd>{String(modelVisible.claim_kind ?? "unknown")}</dd>
    <dt>{t(locale, "reasonCodeLabel")}</dt><dd>{String(finding.reason_code ?? "unknown")}</dd>
  </dl>;
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
  const families = Array.isArray(data.families) ? data.families.map(asObj) : [];
  if (failure) {
    return (
      <p role="alert">
        {t(locale, "reasonCodeLabel")}: <code>{failure}</code>
      </p>
    );
  }
  // Counts come straight from the catalog payload: the daemon-declared
  // `evidence_capability` axis, with a read of `native_oracle`/`reason_code`
  // as the fallback for older daemons. No number is invented here.
  const capabilityOf = (f: Json): string => {
    if (typeof f.evidence_capability === "string") {
      return ["native", "static-only", "connector-required", "unsupported"].includes(f.evidence_capability)
        ? f.evidence_capability : "unknown";
    }
    if (typeof f.native_oracle === "string" && f.native_oracle !== "none-declared-repeatable") return "native";
    if (f.reason_code === "static-resolver-available") return "static-only";
    return f.reason_code === "connector_required" ? "connector-required" : "unsupported";
  };
  const nativeCount = families.filter((f) => capabilityOf(f) === "native").length;
  const staticCount = families.filter((f) => capabilityOf(f) === "static-only").length;
  const connectorCount = families.filter((f) => capabilityOf(f) === "connector-required").length;
  const unsupportedCount = families.filter((f) => capabilityOf(f) === "unsupported").length;
  const unknownCount = families.filter((f) => capabilityOf(f) === "unknown").length;
  const total = families.length;
  return (
    <div>
      {total > 0 ? (
        <>
          <div className="covbar" aria-hidden="true">
            <i className="n" style={{ width: `${(nativeCount / total) * 100}%` }} />
            <i className="s" style={{ width: `${(staticCount / total) * 100}%` }} />
            <i className="c" style={{ width: `${(connectorCount / total) * 100}%` }} />
            <i className="u" style={{ width: `${(unsupportedCount / total) * 100}%` }} />
            <i className="unknown" style={{ width: `${(unknownCount / total) * 100}%` }} />
          </div>
          <div className="cov-legend">
            {unknownCount > 0 ? <span>{t(locale, "unknown")} <b>{unknownCount}</b></span> : null}
            <span>
              <span className="dot" style={{ background: "var(--verified)" }} />
              {t(locale, "covNative")} <b>{nativeCount}</b>
            </span>
            <span>
              <span className="dot" style={{ background: "var(--info)" }} />
              {t(locale, "covStaticOnly")} <b>{staticCount}</b>
            </span>
            <span>
              <span className="dot" style={{ background: "#C7CDD8" }} />
              {t(locale, "covNeedsConnector")} <b>{connectorCount}</b>
            </span>
            {unsupportedCount > 0 ? (
              <span>
                <span className="dot" style={{ background: "var(--text-faint)" }} />
                {t(locale, "covUnsupported")} <b>{unsupportedCount}</b>
              </span>
            ) : null}
          </div>
        </>
      ) : null}
      <div className="row" data-testid="adapter-coverage">
        {families.map((f) => (
          <span key={String(f.family_id)} className="pill" data-family={String(f.family_id)}>
            {String(f.family_name)}
          </span>
        ))}
      </div>
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
      <div className="eyebrow">{t(locale, "navGroupDiagnose")}</div>
      <h1>{t(locale, "checkup")}</h1>
      <p className="page-sub">{t(locale, "checkupLead")}</p>
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
          const truth = String(claim.truth_state ?? "unknown");
          return (
            <div key={name} className={truth === "indeterminate" ? "facet broken" : "facet"}>
              <div className="f-lab">{name}</div>
              <div className={truthClass(truth) ? `f-val ${truthClass(truth)}` : "f-val"}>{truth}</div>
              <div className="f-axis">
                {t(locale, "provenance")}: {String(claim.provenance ?? "")}
              </div>
              <div className="f-axis">
                {t(locale, "coverage")}: {String(claim.coverage ?? "")}
              </div>
              <div className="f-axis">
                {t(locale, "precision")}: {String(claim.precision ?? "")}
              </div>
              <div className="f-axis">
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
      <div className="eyebrow">{t(locale, "navGroupDiagnose")}</div>
      <h1>{t(locale, "compare")}</h1>
      {ids.length < 2 ? <p className="muted">{t(locale, "compareNeedTwo")}</p> : null}
      <SharedStateBanner
        route="/compare"
        verdict={verdict}
        locale={locale}
        onRetry={() => void runDiff()}
      />
      {err ? <p role="alert">{err}</p> : null}
      <div className="sec-head">
        <h2>{t(locale, "runDiff")}</h2>
      </div>
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

/**
 * Whether the viewport is below the C06 narrow breakpoint.
 *
 * Kept in sync with `app.css`: one breakpoint value, two consumers.
 */
const NARROW_QUERY = "(max-width: 767px)";

function useNarrowViewport(): boolean {
  const [narrow, setNarrow] = useState(
    () => typeof window !== "undefined" && window.matchMedia(NARROW_QUERY).matches,
  );
  useEffect(() => {
    const query = window.matchMedia(NARROW_QUERY);
    // Read the query rather than the event, and listen on `resize` as well:
    // a viewport change that does not deliver a `change` event would
    // otherwise strand the user in the wrong layout until a reload.
    const sync = () => setNarrow(query.matches);
    query.addEventListener("change", sync);
    window.addEventListener("resize", sync);
    sync();
    return () => {
      query.removeEventListener("change", sync);
      window.removeEventListener("resize", sync);
    };
  }, []);
  return narrow;
}

/**
 * The C06 narrow viewport: read-only Receipts and notifications.
 *
 * Below 768px the full application is not offered. Previously the navigation
 * was simply hidden, which left the whole app rendered with no way to move
 * between pages; this view is the read-only surface the spec actually asks
 * for, and it says so rather than looking like a degraded full app.
 */
function NarrowReadOnly({
  locale,
  privacy,
  hold,
  onCopy,
}: {
  locale: Locale;
  privacy: "default" | "screenshot";
  hold: boolean;
  onCopy: (event: ClipboardEvent) => void;
}) {
  const list = useResource("/api/v1/receipts");
  const [selected, setSelected] = useState("");
  const detail = useResource(selected ? `/api/v1/receipts/${selected}` : "/api/v1/health");
  const rows = useMemo(() => {
    const items = asObj(list.data).receipts;
    return Array.isArray(items) ? (items as Json[]) : [];
  }, [list.data]);

  return (
    <main className="narrow" id="main-content" data-privacy={privacy} lang={locale}>
      <header className="panel">
        <h1>{t(locale, "narrowTitle")}</h1>
        <p className="muted">{t(locale, "narrowReadOnly")}</p>
      </header>

      <section className="panel">
        <h2>{t(locale, "receipts")}</h2>
        <StateBanner
          route="/receipts"
          status={list.status}
          reasonCode={list.reasonCode}
          retryable={list.retryable}
          locale={locale}
          onRetry={list.retry}
        />
        {rows.length === 0 ? null : (
          <ul>
            {rows.map((row) => {
              const id = String(row.receipt_id ?? "");
              return (
                <li key={id}>
                  <button
                    type="button"
                    aria-pressed={selected === id}
                    onClick={() => setSelected(id === selected ? "" : id)}
                  >
                    {id} · {String(row.receipt_kind ?? "")}
                    {/* A tombstone is not a Receipt; the list must not read
                        as if the record is still there. */}
                    {row.tombstone === true ? ` · ${t(locale, "receiptTombstoned")}` : ""}
                  </button>
                </li>
              );
            })}
          </ul>
        )}
        {selected ? (
          <MaskedText
            text={JSON.stringify(detail.data, null, 2)}
            hold={hold}
            locale={locale}
            onCopy={onCopy}
          />
        ) : null}
      </section>

      <section className="panel">
        <h2>{t(locale, "notifications")}</h2>
        {/* No endpoint writes or serves notifications in this slice; saying
            "none" would be indistinguishable from "none yet". */}
        <p role="status" data-testid="narrow-notifications">
          {t(locale, "narrowNotificationsUnimplemented")} · {t(locale, "reasonCodeLabel")}:{" "}
          <code>notifications.unimplemented</code>
        </p>
      </section>
    </main>
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
  // Abort stops the work; the generation stops a late answer that already
  // left the daemon from touching a newer request's state (C01/C40).
  const generation = useRef(createGeneration());

  useEffect(() => {
    const ctrl = new AbortController();
    controller.current = ctrl;
    const mine = generation.current.next();
    setState({ status: "loading", reasonCode: "", retryable: false, data: null });
    void requestJson(path, { signal: ctrl.signal }).then((result) => {
      if (!generation.current.isCurrent(mine)) return;
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
export function StateBanner({
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
    <div className="statebanner" role={isFailure ? "alert" : "status"} data-state={status}>
      <div>
        <span className="sb-title">{t(locale, labelKey)}</span>
        {reasonCode ? (
          <span className="sb-body">
            {" "}
            · {t(locale, "reasonCodeLabel")}: <code>{reasonCode}</code>
          </span>
        ) : null}
        <p className="sb-body">
          {t(locale, "nextStepLabel")}: {t(locale, STATE_NEXT[status] ?? "nextError")}
        </p>
        {!declared ? (
          // The contract said this page could not reach this state. Surface the
          // contradiction rather than hiding it behind a generic message.
          <p className="sb-body" data-undeclared="true">{t(locale, "stateNotApplicable")}</p>
        ) : null}
      </div>
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
 * Pages without a `render` function still show the raw JSON dump. Pages with
 * one own their presentation entirely — including how (or whether) the raw
 * payload stays reachable — so a structured view never reads as if the dump
 * were the content.
 */
function StateView({
  path,
  route,
  title,
  locale,
  headingLevel = 1,
  render,
}: {
  path: string;
  route: string;
  title: string;
  locale: Locale;
  /** Use 2 when this view sits inside a page that already has an `h1`;
      two `h1`s in one document make the outline ambiguous (SC 1.3.1). */
  headingLevel?: 1 | 2;
  /** A presentation for the payload. When given, it replaces the JSON dump. */
  render?: (data: Json) => ReactNode;
}) {
  const res = useResource(path);
  const Heading = headingLevel === 2 ? "h2" : "h1";
  return (
    <section className="panel">
      <Heading>{title}</Heading>
      {res.status === "loading" ? (
        <p role="status" data-state="loading">
          <span className="loading-pulse">{t(locale, "loading")}</span>{" "}
          <button type="button" onClick={res.cancel}>
            {t(locale, "cancel")}
          </button>
        </p>
      ) : null}
      {res.status === "cancelled" ? (
        <p role="status" data-state="cancelled">
          {t(locale, "cancelled")} <code>{res.reasonCode}</code>{" "}
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
        render ? (
          render(asObj(res.data))
        ) : (
          <pre className="mono">{JSON.stringify(res.data, null, 2)}</pre>
        )
      ) : null}
    </section>
  );
}

/** The raw payload, folded away but still one click from the summary. */
function RawJsonDetails({ data, locale }: { data: unknown; locale: Locale }) {
  return (
    <details>
      <summary className="muted">{t(locale, "rawPayload")}</summary>
      <pre className="mono">{JSON.stringify(data, null, 2)}</pre>
    </details>
  );
}

/** Effect Lab: the list says which experiments were executed at all. */
export function LabListView({ data, locale }: { data: Json; locale: Locale }) {
  const items = Array.isArray(data.experiments) ? data.experiments : [];
  return (
    <>
      <table>
        <thead>
          <tr>
            <th>experiment_id</th>
            <th>{t(locale, "labExecuted")}</th>
            <th>{t(locale, "labDecision")}</th>
            <th>{t(locale, "reasonCodeLabel")}</th>
          </tr>
        </thead>
        <tbody>
          {items.length === 0 ? (
            <tr>
              <td colSpan={4} className="muted">
                {t(locale, "labEmpty")}
              </td>
            </tr>
          ) : (
            items.map((raw) => {
              const item = asObj(raw);
              const id = String(item.experiment_id ?? "");
              const executed = item.executed === true;
              return (
                <tr key={id} data-executed={String(executed)}>
                  <td>
                    <NavLink to={`/lab/${encodeURIComponent(id)}`}>{id}</NavLink>
                  </td>
                  <td>{executed ? t(locale, "labExecuted") : t(locale, "labNotExecuted")}</td>
                  <td>{item.decision == null ? "—" : String(item.decision)}</td>
                  <td>
                    <code>{String(item.reason_code ?? "")}</code>
                  </td>
                </tr>
              );
            })
          )}
        </tbody>
      </table>
      <RawJsonDetails data={data} locale={locale} />
    </>
  );
}

/**
 * V04 Receipts: the store index rows — id, kind, harness, creation time,
 * manifest digest (prefix) and tombstone flag — as served by
 * `GET /api/v1/receipts`. An unrecognized payload falls back to the raw
 * dump rather than guessed-at columns.
 */
function ReceiptsListView({ data, locale }: { data: Json; locale: Locale }) {
  if (!Array.isArray(data.receipts)) {
    return <pre className="mono">{JSON.stringify(data, null, 2)}</pre>;
  }
  const rows = data.receipts.map(asObj);
  return (
    <>
      <table>
        <thead>
          <tr>
            <th>receipt_id</th>
            <th>receipt_kind</th>
            <th>harness</th>
            <th>created_at</th>
            <th>digest</th>
            <th>tombstone</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={6} className="muted">
                {t(locale, "empty")}
              </td>
            </tr>
          ) : (
            rows.map((row) => {
              const id = String(row.receipt_id ?? "");
              return (
                <tr key={id} data-tombstone={String(row.tombstone === true)}>
                  <td>
                    <NavLink className="mono" to={`/receipts/${encodeURIComponent(id)}`}>
                      {id}
                    </NavLink>
                  </td>
                  <td>{String(row.receipt_kind ?? "")}</td>
                  <td>{String(row.harness ?? "")}</td>
                  <td className="mono">{String(row.created_at ?? "")}</td>
                  <td>
                    <code>{String(row.digest ?? "").slice(0, 16)}</code>
                  </td>
                  <td>{row.tombstone === true ? t(locale, "receiptTombstoned") : "—"}</td>
                </tr>
              );
            })
          )}
        </tbody>
      </table>
      <RawJsonDetails data={data} locale={locale} />
    </>
  );
}

/**
 * V06 Sessions: the list endpoint serves ids only (`{sessions: [id]}`), so
 * the table has exactly one honest column; everything else lives on the
 * per-session page.
 */
function MonitorView({ data, locale }: { data: Json; locale: Locale }) {
  if (data.schema !== "ctxpect-monitor-v1") return <pre className="mono">{JSON.stringify(data, null, 2)}</pre>;
  const staleness = asObj(data.staleness);
  return <>
    <dl className="kv" data-testid="monitor-summary">
      <dt>Receipt</dt><dd>{String(data.current_receipt_id ?? "—")}</dd>
      <dt>{t(locale, "freshness")}</dt><dd>{String(staleness.status ?? "unknown")}</dd>
      <dt>{t(locale, "reasonCodeLabel")}</dt><dd>{String(staleness.reason_code ?? "—")}</dd>
      <dt>mode</dt><dd>{String(data.mode ?? "—")}</dd>
      <dt>compared</dt><dd>{String(staleness.compared ?? "—")}</dd>
    </dl>
    <RawJsonDetails data={data} locale={locale} />
  </>;
}

function SessionsListView({ data, locale }: { data: Json; locale: Locale }) {
  if (!Array.isArray(data.sessions)) {
    return <pre className="mono">{JSON.stringify(data, null, 2)}</pre>;
  }
  const summaries = Array.isArray(data.session_summaries) ? data.session_summaries.map(asObj) : [];
  const ids = data.sessions.filter((id): id is string => typeof id === "string");
  return <>
    <table data-testid="session-list">
      <thead><tr><th>session_id</th><th>mapping_id</th><th>event_count</th><th>partial</th><th>bodies_stored</th></tr></thead>
      <tbody>{ids.length === 0 ? <tr><td colSpan={5}>{t(locale, "empty")}</td></tr> : ids.map((id) => {
        const summary = summaries.find((item) => item.session_id === id) ?? {};
        return <tr key={id}>
          <td><NavLink className="mono" to={`/sessions/${encodeURIComponent(id)}`}>{id}</NavLink></td>
          <td>{String(summary.mapping_id ?? "—")}</td>
          <td>{String(summary.event_count ?? "—")}</td>
          <td>{typeof summary.partial === "boolean" ? t(locale, summary.partial ? "boolYes" : "boolNo") : "—"}</td>
          <td>{typeof summary.bodies_stored === "boolean" ? t(locale, summary.bodies_stored ? "boolYes" : "boolNo") : "—"}</td>
        </tr>;
      })}</tbody>
    </table>
    <RawJsonDetails data={data} locale={locale} />
  </>;
}

/**
 * V13 Exceptions: the list endpoint serves ids only (`{exceptions: [id]}`),
 * and the UI has no /exceptions/:id route, so ids render as text rather
 * than as links to a route that does not exist.
 */
function ExceptionsListView({ data, locale }: { data: Json; locale: Locale }) {
  if (!Array.isArray(data.exceptions)) {
    return <pre className="mono">{JSON.stringify(data, null, 2)}</pre>;
  }
  const ids = data.exceptions.map(String);
  return (
    <>
      <p className="muted">{t(locale, "exceptionsCliNote")}</p>
      <table>
        <thead>
          <tr>
            <th>exception_id</th>
          </tr>
        </thead>
        <tbody>
          {ids.length === 0 ? (
            <tr>
              <td className="muted">{t(locale, "empty")}</td>
            </tr>
          ) : (
            ids.map((id) => (
              <tr key={id}>
                <td className="mono">{id}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
      <RawJsonDetails data={data} locale={locale} />
    </>
  );
}

/**
 * V10 Policy: the effective mutation decision for one scope.
 *
 * The pill keeps the machine verdict visible; colour is only a coarse
 * grouping (pass → verified hue, deny/fail-closed → confirmed hue,
 * unknown/indeterminate → unknown hue, detect-only/approval_required →
 * suspected hue). detect-only is never drawn as a pass, and a payload
 * without layers is Unknown, not a pass.
 */
function PolicyView({ data, locale }: { data: Json; locale: Locale }) {
  if (data.schema !== "ctxpect-policy-effective-v1") {
    return <pre className="mono">{JSON.stringify(data, null, 2)}</pre>;
  }
  const verdict = String(data.verdict ?? "unknown");
  const verdictClass =
    verdict === "pass"
      ? "native"
      : verdict === "deny" || verdict === "fail-closed"
        ? "confirmed"
        : verdict === "unknown" || verdict === "indeterminate"
          ? "unknown"
          : "suspected";
  const boolCell = (v: unknown) =>
    typeof v === "boolean" ? t(locale, v ? "boolYes" : "boolNo") : "—";
  const scope = asObj(data.scope);
  const layersPresent = data.layers_present === true;
  const evaluation = asObj(data.layer_evaluation);
  const hasEvaluation = evaluation.schema === "ctxpect-policy-eval-v1";
  const ruleGroups: { group: string; pill: string; rules: Json[] }[] = hasEvaluation
    ? [
        {
          group: "required",
          pill: "confirmed",
          rules: Array.isArray(evaluation.required_rules) ? evaluation.required_rules.map(asObj) : [],
        },
        {
          group: "required-allow",
          pill: "native",
          rules: Array.isArray(evaluation.required_allow_rules)
            ? evaluation.required_allow_rules.map(asObj)
            : [],
        },
        {
          group: "detect-only",
          pill: "suspected",
          rules: Array.isArray(evaluation.detect_only_rules) ? evaluation.detect_only_rules.map(asObj) : [],
        },
      ]
    : [];
  const ruleCount = ruleGroups.reduce((sum, group) => sum + group.rules.length, 0);
  return (
    <>
      <p>
        <span className={`pill ${verdictClass}`}>
          {t(locale, "decision")}: <span className="mono">{verdict}</span>
        </span>{" "}
        {t(locale, "reasonCodeLabel")}: <code>{String(data.reason_code ?? "")}</code>
      </p>
      <dl className="kv">
        <dt>scope.action</dt>
        <dd>{String(scope.action ?? "—")}</dd>
        <dt>scope.target</dt>
        <dd>{String(scope.target ?? "—")}</dd>
        <dt>scope.project_digest</dt>
        <dd>{String(scope.project_digest ?? "—")}</dd>
        <dt>mutation_allowed</dt>
        <dd>{boolCell(data.mutation_allowed)}</dd>
        <dt>layers_present</dt>
        <dd>{boolCell(data.layers_present)}</dd>
        <dt>policy_source_fresh</dt>
        <dd>{boolCell(data.policy_source_fresh)}</dd>
        <dt>exception_id</dt>
        <dd>{typeof data.exception_id === "string" && data.exception_id ? data.exception_id : "—"}</dd>
        {typeof data.message === "string" && data.message ? (
          <>
            <dt>message</dt>
            <dd>{data.message}</dd>
          </>
        ) : null}
      </dl>
      {layersPresent ? null : (
        // No policy layers means nothing can be enforced; the contract
        // answer here is Unknown, never a pass.
        <p className="muted">layers_present: {t(locale, "boolNo")} — {t(locale, "unknown")}</p>
      )}
      {hasEvaluation ? (
        <>
          <dl className="kv">
            <dt>evaluation.verdict</dt>
            <dd className="mono">{String(evaluation.verdict ?? "—")}</dd>
            <dt>evaluation.enforceable</dt>
            <dd>{boolCell(evaluation.enforceable)}</dd>
            <dt>evaluation.unique_authority</dt>
            <dd>{String(evaluation.unique_authority ?? "—")}</dd>
          </dl>
          <table>
            <thead>
              <tr>
                <th>rule</th>
                <th>layer</th>
                <th>effect</th>
                <th>required</th>
                <th>group</th>
              </tr>
            </thead>
            <tbody>
              {ruleCount === 0 ? (
                <tr>
                  <td colSpan={5} className="muted">
                    {t(locale, "empty")}
                  </td>
                </tr>
              ) : (
                ruleGroups.flatMap((group) =>
                  group.rules.map((rule, index) => (
                    <tr key={`${group.group}-${String(rule.id ?? index)}`}>
                      <td className="mono">{String(rule.id ?? "—")}</td>
                      <td>{String(rule.layer ?? "—")}</td>
                      <td>{String(rule.effect ?? "—")}</td>
                      <td>{boolCell(rule.required)}</td>
                      <td>
                        <span className={`pill ${group.pill}`}>{group.group}</span>
                      </td>
                    </tr>
                  )),
                )
              )}
            </tbody>
          </table>
        </>
      ) : (
        <p className="muted">layer_evaluation: —</p>
      )}
      <RawJsonDetails data={data} locale={locale} />
    </>
  );
}

/**
 * V14 Team compliance: a metadata-only roll-up of the local store. The
 * payload's own disclosure leads the page, and drift/unknown stay "—" when
 * no Receipt exists rather than reading as zero.
 */
function TeamComplianceView({ data, locale }: { data: Json; locale: Locale }) {
  if (data.schema !== "ctxpect-team-compliance-v1") {
    return <pre className="mono">{JSON.stringify(data, null, 2)}</pre>;
  }
  const boolCell = (v: unknown) =>
    typeof v === "boolean" ? t(locale, v ? "boolYes" : "boolNo") : "—";
  const num = (v: unknown) => (typeof v === "number" ? String(v) : "—");
  const cell = (v: unknown) => (v == null ? "—" : typeof v === "string" ? v : JSON.stringify(v));
  const disclosure = asObj(data.disclosure);
  const standardStatus = asObj(data.standard_status);
  const exceptionInfo = asObj(data.exception);
  const standards = Array.isArray(standardStatus.standards) ? standardStatus.standards.map(asObj) : [];
  const granting = Array.isArray(exceptionInfo.granting) ? exceptionInfo.granting.map(asObj) : [];
  const drift = data.drift == null ? null : asObj(data.drift);
  const freshness = asObj(data.freshness);
  const audit = asObj(data.audit);
  return (
    <>
      <div className="doc-note">
        <strong>{String(disclosure.scope ?? "—")}</strong> · {String(disclosure.note ?? "")}
      </div>
      <dl className="kv">
        <dt>standard_status.total</dt>
        <dd>{num(standardStatus.total)}</dd>
        <dt>standard_status.signed</dt>
        <dd>{num(standardStatus.signed)}</dd>
        <dt>standard_status.adopted</dt>
        <dd>{num(standardStatus.adopted)}</dd>
        <dt>exception.total</dt>
        <dd>{num(exceptionInfo.total)}</dd>
        <dt>exception.live</dt>
        <dd>{num(exceptionInfo.live)}</dd>
      </dl>
      <h2>{t(locale, "standards")}</h2>
      <table>
        <thead>
          <tr>
            <th>standard_id</th>
            <th>signed</th>
            <th>adoption_state</th>
            <th>pinned_digest</th>
          </tr>
        </thead>
        <tbody>
          {standards.length === 0 ? (
            <tr>
              <td colSpan={4} className="muted">
                {t(locale, "empty")}
              </td>
            </tr>
          ) : (
            standards.map((standard) => (
              <tr key={String(standard.standard_id)}>
                <td className="mono">{String(standard.standard_id ?? "—")}</td>
                <td>{boolCell(standard.signed)}</td>
                <td>{String(standard.adoption_state ?? "—")}</td>
                <td>
                  {typeof standard.pinned_digest === "string" ? (
                    <code>{standard.pinned_digest.slice(0, 16)}</code>
                  ) : (
                    "—"
                  )}
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
      <h2>{t(locale, "exceptions")}</h2>
      <table>
        <thead>
          <tr>
            <th>exception_id</th>
            <th>requester</th>
            <th>decided_by</th>
          </tr>
        </thead>
        <tbody>
          {granting.length === 0 ? (
            <tr>
              <td colSpan={3} className="muted">
                {t(locale, "empty")}
              </td>
            </tr>
          ) : (
            granting.map((grant) => (
              <tr key={String(grant.exception_id)}>
                <td className="mono">{String(grant.exception_id ?? "—")}</td>
                <td>{cell(grant.requester)}</td>
                <td>{cell(grant.decided_by)}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>
      <h2>drift</h2>
      {drift ? (
        <dl className="kv">
          <dt>confirmed</dt>
          <dd>{num(drift.confirmed)}</dd>
          <dt>suspected</dt>
          <dd>{num(drift.suspected)}</dd>
          <dt>unknown</dt>
          <dd>{num(drift.unknown)}</dd>
        </dl>
      ) : (
        <p className="muted">
          —
          {typeof freshness.reason_code === "string" && freshness.reason_code ? (
            <>
              {" "}
              · {t(locale, "reasonCodeLabel")}: <code>{freshness.reason_code}</code>
            </>
          ) : null}
        </p>
      )}
      <h2>audit</h2>
      <dl className="kv">
        <dt>verified</dt>
        <dd>{boolCell(audit.verified)}</dd>
        <dt>count</dt>
        <dd>{num(audit.count)}</dd>
        <dt>legacy_entries</dt>
        <dd>{num(audit.legacy_entries)}</dd>
        <dt>reason_code</dt>
        <dd>
          {typeof audit.reason_code === "string" && audit.reason_code ? (
            <code>{audit.reason_code}</code>
          ) : (
            "—"
          )}
        </dd>
      </dl>
      <RawJsonDetails data={data} locale={locale} />
    </>
  );
}

/**
 * Effect Lab: one experiment. An experiment without per-run results was not
 * executed and has no decision; the page says so before anything else.
 */
export function LabResultView({ data, locale }: { data: Json; locale: Locale }) {
  const executed = data.executed === true;
  const estimator = asObj(data.estimator);
  return (
    <div data-testid="lab-result" data-executed={String(executed)}>
      <p role="status">
        <strong>{executed ? t(locale, "labExecuted") : t(locale, "labNotExecuted")}</strong>
        {" · "}
        {t(locale, "labDecision")}: {data.decision == null ? "—" : String(data.decision)}
        {" · "}
        {t(locale, "reasonCodeLabel")}: <code>{String(data.reason_code ?? "")}</code>
      </p>
      {typeof data.note === "string" ? <p>{data.note}</p> : null}
      <p>
        {t(locale, "labEstimator")}: {estimator.available === true ? String(estimator.name ?? "") : t(locale, "unknown")}
        {typeof estimator.note === "string" ? ` — ${estimator.note}` : null}
      </p>
    </div>
  );
}

type TimelineRow = { seq: number; type: string; len: number; digest: string; surface_op: string | null };

function timelineRows(session: Json): TimelineRow[] {
  const rows = Array.isArray(session.timeline) ? session.timeline : [];
  return rows.map((raw) => {
    const row = asObj(raw);
    return {
      seq: Number(row.seq ?? -1),
      type: String(row.type ?? ""),
      len: Number(row.len ?? 0),
      digest: String(row.digest ?? ""),
      surface_op: typeof row.surface_op === "string" ? row.surface_op : null,
    };
  });
}

function fmtRanges(value: unknown): string {
  if (!Array.isArray(value) || value.length === 0) return "—";
  return value
    .map((item) => {
      if (Array.isArray(item) && item.length === 2) {
        return item[0] === item[1] ? String(item[0]) : `${item[0]}–${item[1]}`;
      }
      const r = asObj(item);
      return `${r.seq}: ${r.start}–${r.end}`;
    })
    .join(", ");
}

/** Rows of type/seq/len/digest only: no body ever reaches this table. */
function MetadataRows({ rows, caption }: { rows: TimelineRow[]; caption: string }) {
  return (
    <table data-metadata-only="true">
      <caption>{caption}</caption>
      <thead>
        <tr>
          <th>seq</th>
          <th>type</th>
          <th>len</th>
          <th>digest</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.seq}>
            <td>{row.seq}</td>
            <td>{row.type}</td>
            <td>{row.len}</td>
            <td>
              <code>{row.digest.slice(0, 16)}</code>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/**
 * V06 request evidence: what the store knows about each request an
 * imported session made — header digest, message count, source and
 * replacement ranges, dispatch evidence — and two metadata-only views: the
 * human-visible history (append-origin surface events) and the surface the
 * selected request was derived from. Nothing here renders a body.
 */
export function SessionRequestsView({
  session,
  requests,
  selected,
  onSelect,
  locale,
}: {
  session: Json;
  requests: Json;
  selected: number | null;
  onSelect: (index: number) => void;
  locale: Locale;
}) {
  const rows = timelineRows(session);
  const bySeq = new Map(rows.map((row) => [row.seq, row]));
  const list = Array.isArray(requests.requests) ? requests.requests.map(asObj) : [];
  const tail = asObj(requests.tail);
  const chosen = selected != null ? list[selected] : undefined;
  const derived: TimelineRow[] = chosen
    ? (Array.isArray(chosen.surface_nodes) ? chosen.surface_nodes : [])
        .map((seq) => bySeq.get(Number(seq)))
        .filter((row): row is TimelineRow => row !== undefined)
    : [];
  const human = rows.filter((row) => row.surface_op === "append");
  return (
    <div data-testid="session-requests">
      <p>
        {t(locale, "sessionNoBodies")}
        {requests.partial === true ? ` · ${t(locale, "sessionPartial")}` : null}
      </p>
      <table data-testid="request-table">
        <caption>{t(locale, "sessionRequests")}</caption>
        <thead>
          <tr>
            <th>seq</th>
            <th>reason</th>
            <th>{t(locale, "sessionRequestHeader")}</th>
            <th>{t(locale, "sessionRequestMessages")}</th>
            <th>{t(locale, "sessionRequestSources")}</th>
            <th>{t(locale, "sessionRequestReplaced")}</th>
            <th>{t(locale, "sessionRequestDispatch")}</th>
            <th>{t(locale, "sessionRequestLimits")}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {list.map((request, index) => {
            const reasons = Array.isArray(request.unknown_reasons) ? request.unknown_reasons : [];
            const dispatch = request.dispatch_evidence;
            return (
              <tr key={String(request.seq)} data-selected={String(selected === index)}>
                <td>{String(request.seq)}</td>
                <td>{String(request.reason ?? "")}</td>
                <td>
                  <code>{String(request.header_digest ?? "").slice(0, 16)}</code>
                </td>
                <td>{String(request.message_count ?? 0)}</td>
                <td>{fmtRanges(request.source_seq_ranges)}</td>
                <td>{fmtRanges(request.replaced_ranges)}</td>
                <td data-dispatch={dispatch == null ? "none" : "seq"}>
                  {dispatch == null ? t(locale, "sessionRequestPreparedOnly") : `seq ${String(dispatch)}`}
                </td>
                <td>
                  {reasons.length === 0 ? (
                    "—"
                  ) : (
                    <ul>
                      {reasons.map((reason) => (
                        <li key={String(reason)}>{String(reason)}</li>
                      ))}
                    </ul>
                  )}
                </td>
                <td>
                  <button type="button" onClick={() => onSelect(index)} aria-pressed={selected === index}>
                    {t(locale, "sessionSelectRequest")}
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <section data-testid="session-tail">
        <h2>{t(locale, "sessionTail")}</h2>
        <p>{tail.interrupted === true ? t(locale, "sessionTailInterrupted") : t(locale, "sessionTailBalanced")}</p>
        {Array.isArray(tail.closers) && tail.closers.length > 0 ? (
          <ul>
            {tail.closers.map((raw, index) => {
              const closer = asObj(raw);
              return (
                <li key={index}>
                  {String(closer.type)}
                  {typeof closer.error_code === "string" ? ` · ${closer.error_code}` : null}
                  {typeof closer.reason === "string" ? ` · ${closer.reason}` : null}
                </li>
              );
            })}
          </ul>
        ) : null}
      </section>
      <MetadataRows rows={human} caption={t(locale, "sessionHumanHistory")} />
      {chosen ? (
        <MetadataRows
          rows={derived}
          caption={`${t(locale, "sessionDerivedSurface")} · seq ${String(chosen.seq)}`}
        />
      ) : null}
    </div>
  );
}

/**
 * `/sessions/:id`: two GETs (the session record, its request evidence) under
 * one generation. Switching to another session bumps the generation, so a
 * late response for the previous id is dropped instead of being rendered
 * under the new one (the inspect chain's C01/C40 rule, reused).
 */
function SessionRequestsPage({ locale }: { locale: Locale }) {
  const { id } = useParams();
  const generation = useRef(createGeneration());
  const [session, setSession] = useState<Json | null>(null);
  const [requests, setRequests] = useState<Json | null>(null);
  const [verdict, setVerdict] = useState<StateVerdict & { status: string }>({
    status: "loading",
    state: "loading",
    reasonCode: "",
    retryable: false,
  });
  const [selected, setSelected] = useState<number | null>(null);
  const [attempt, setAttempt] = useState(0);
  const controller = useRef<AbortController | null>(null);

  useEffect(() => {
    if (!id) return undefined;
    const gen = generation.current.next();
    const ctrl = new AbortController();
    controller.current = ctrl;
    setSession(null);
    setRequests(null);
    setSelected(null);
    setVerdict({ status: "loading", state: "loading", reasonCode: "", retryable: false });
    const base = `/api/v1/sessions/${encodeURIComponent(id)}`;
    void Promise.all([
      requestJson(base, { signal: ctrl.signal }),
      requestJson(`${base}/requests`, { signal: ctrl.signal }),
    ]).then(([doc, reqs]) => {
      if (!generation.current.isCurrent(gen)) return;
      if (!doc.ok || !reqs.ok) {
        const failed = doc.ok ? reqs : doc;
        if (failed.ok) return;
        if (failed.code === "api.cancelled") {
          setVerdict({ status: "cancelled", state: "cancelled", reasonCode: failed.code, retryable: true });
          return;
        }
        const v = classifyFailure(failed.kind, failed.code);
        setVerdict({ ...v, status: v.state });
        return;
      }
      const v = classifyPayload(reqs.data);
      setSession(asObj(doc.data));
      setRequests(asObj(reqs.data));
      setVerdict({ ...v, status: v.state });
    });
    return () => ctrl.abort();
  }, [id, attempt]);

  if (!id) {
    return (
      <section className="panel">
        <h1>{t(locale, "sessions")}</h1>
        <p role="alert">{t(locale, "missingId")}</p>
      </section>
    );
  }
  return (
    <section className="panel">
      <h1>
        {t(locale, "sessions")} / {id}
      </h1>
      {verdict.status === "loading" ? (
        <p role="status">
          <span className="loading-pulse">{t(locale, "loading")}</span>{" "}
          <button type="button" onClick={() => controller.current?.abort()}>
            {t(locale, "cancel")}
          </button>
        </p>
      ) : null}
      {verdict.status === "cancelled" ? (
        <p role="status">
          {t(locale, "cancelled")}{" "}
          <button type="button" onClick={() => setAttempt((n) => n + 1)}>
            {t(locale, "retry")}
          </button>
        </p>
      ) : null}
      <StateBanner
        route="/sessions"
        status={verdict.status}
        reasonCode={verdict.reasonCode}
        retryable={verdict.retryable}
        locale={locale}
        onRetry={() => setAttempt((n) => n + 1)}
      />
      {session && requests ? (
        <SessionRequestsView
          session={session}
          requests={requests}
          selected={selected}
          onSelect={setSelected}
          locale={locale}
        />
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

  const meta = asObj(res.data);
  return (
    <section className="docpage">
      <div className="doc-eyebrow">{t(locale, "receiptDocEyebrow")}</div>
      <h1 className="doc-title">{id}</h1>
      <div className="doc-meta">
        <span>{t(locale, "receipts")}</span>
        {typeof meta.receipt_kind === "string" && meta.receipt_kind ? (
          <span>
            kind · <b>{meta.receipt_kind}</b>
          </span>
        ) : null}
        {meta.tombstone === true ? (
          <span>
            <b>{t(locale, "receiptTombstoned")}</b>
          </span>
        ) : null}
      </div>
      {res.status === "loading" ? (
        <p role="status" data-state="loading">
          <span className="loading-pulse">{t(locale, "loading")}</span>{" "}
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
        <>
          <dl className="kv">
            <dt>receipt_id</dt>
            <dd>{id}</dd>
            <dt>receipt_kind</dt>
            <dd>{String(meta.receipt_kind ?? "—")}</dd>
            <dt>tombstone</dt>
            <dd>{String(meta.tombstone === true)}</dd>
          </dl>
          <pre className="mono">{JSON.stringify(res.data, null, 2)}</pre>
        </>
      ) : null}
    </section>
  );
}

function EntityPage({
  folder,
  locale,
  render,
}: {
  folder: string;
  locale: Locale;
  render?: (data: Json) => ReactNode;
}) {
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
      render={render}
    />
  );
}

/** One editor control, rendered from the field's published spec. */
/** A stable id for a field's error message, usable in `aria-describedby`. */
function problemId(path: string): string {
  return `settings-error-${path.replace(/\./g, "-")}`;
}

function SettingsField({
  path,
  spec,
  value,
  locale,
  disabled,
  problems,
  unenforced,
  onChange,
}: {
  path: string;
  spec: Json;
  value: unknown;
  locale: Locale;
  disabled: boolean;
  problems: SettingsProblem[];
  /** Reasons, by field path, for settings the store does not act on. */
  unenforced: Json;
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
            problems={problems}
            unenforced={unenforced}
            onChange={onChange}
          />
        ))}
      </fieldset>
    );
  }

  // The field's own problem, if any. Announcing it on the control itself is
  // what lets someone who tabs onto the field know it is wrong — a summary
  // list elsewhere on the page does not do that (WCAG 2.2 SC 3.3.1, 4.1.2).
  // A field the store validates but never acts on must say so, or it reads
  // as a working knob.
  const notEnforced = typeof unenforced[path] === "string" ? String(unenforced[path]) : "";
  const problem = problems.find((item) => item.path === path);
  const invalid = problem !== undefined;
  const describedBy = invalid ? problemId(path) : undefined;
  const notEnforcedNote = notEnforced ? (
    <span className="muted" data-testid={`unenforced-${path}`}>
      {t(locale, "settingsNotEnforced")}: {notEnforced}
    </span>
  ) : null;
  const errorNote = problem ? (
    <span id={problemId(path)} className="field-error">
      <code>{problem.code}</code> — {problem.message}
    </span>
  ) : null;
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
          aria-invalid={invalid || undefined}
          aria-describedby={describedBy}
          onChange={(e) => onChange(path, e.target.checked)}
        />
        {errorNote}
        {notEnforcedNote}
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
          aria-invalid={invalid || undefined}
          aria-describedby={describedBy}
          onChange={(e) => onChange(path, e.target.value)}
        >
          {values.map((item) => (
            <option key={String(item)} value={String(item)}>
              {String(item)}
            </option>
          ))}
        </select>
        {errorNote}
        {notEnforcedNote}
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
          aria-invalid={invalid || undefined}
          aria-describedby={describedBy}
          onChange={(e) => {
            const raw = e.target.value;
            // Keep an empty box distinguishable from 0 so the draft does not
            // silently become a valid-looking value while being edited.
            onChange(path, raw === "" ? raw : Number(raw));
          }}
        />
        {errorNote}
        {notEnforcedNote}
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
      <div className="eyebrow">{t(locale, "navGroupRecords")}</div>
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
        <dl className="kv" data-testid="assets-plan">
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
      <div className="eyebrow">{t(locale, "navGroupActions")}</div>
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
        <>
          <div className="sec-head">
            <h2>
              {t(locale, "syncTransport")} · {t(locale, "syncSemantic")}
            </h2>
            <span className="sec-note">{t(locale, "syncTransportNotVerified")}</span>
          </div>
          <dl className="kv" data-testid="sync-outcome">
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
        </>
      ) : null}

      {status.data != null ? (
        <pre className="mono">{JSON.stringify(status.data, null, 2)}</pre>
      ) : null}
    </section>
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
function SettingsPage({ locale }: { locale: Locale }) {
  const [schema, setSchema] = useState<Json>({});
  const [unenforced, setUnenforced] = useState<Json>({});
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
    setUnenforced(asObj(asObj(schemaResult.data).unenforced));
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
      <div className="eyebrow">{t(locale, "navGroupGovernance")}</div>
      <h1>{t(locale, "settings")}</h1>
      <p className="page-sub">{t(locale, "vaultDefault")}</p>
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
                problems={problems}
                unenforced={unenforced}
                onChange={(path, value) => setDraft((current) => setPath(current, path, value))}
              />
            ))}
          </div>

          {problems.length > 0 ? (
            // A count, not a repeat of each message: the messages already sit
            // on their fields, and announcing them twice is noise.
            <p role="alert" data-testid="settings-problems">
              {t(locale, "settingsProblemCount")}: {problems.length}
            </p>
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

function CarePlanPage({ locale, receiptQuery }: { locale: Locale; receiptQuery: string }) {
  const { findingId } = useParams();
  return (
    <section className="panel">
      <h1>{t(locale, "carePlan")}</h1>
      <p>
        {t(locale, "carePlanFinding")} {findingId}
      </p>
      <p>{t(locale, "carePlanLocked")}</p>
      <StateView
        path={`/api/v1/care-plan/${findingId ?? ""}${receiptQuery}`}
        route="/care-plan/:findingId"
        title={t(locale, "plan")}
        locale={locale}
        headingLevel={2}
      />
    </section>
  );
}

function IntegrationsPage({ locale }: { locale: Locale }) {
  return (
    <section className="panel">
      <div className="eyebrow">{t(locale, "navGroupActions")}</div>
      <h1>{t(locale, "integrations")}</h1>
      <p className="page-sub">{t(locale, "integrationsIndependence")}</p>
      <AdapterCoverage locale={locale} />
      <StateView
        path="/api/v1/integrations"
        route="/integrations"
        title={t(locale, "catalog")}
        locale={locale}
        headingLevel={2}
      />
    </section>
  );
}
