// Settings editor logic: what changed, and what the store will refuse.
//
// The store is the authority — `put_settings` validates every write. This
// module mirrors those rules so the editor can say why a value is wrong
// before a round trip, and it emits the *same* reason codes so a client-side
// message and a server-side one never disagree about what went wrong.

/** Fields whose value differs between the saved document and the draft. */
export function changedFields(saved, draft, prefix = "") {
  const out = [];
  const keys = new Set([...Object.keys(saved ?? {}), ...Object.keys(draft ?? {})]);
  for (const key of keys) {
    const path = prefix ? `${prefix}.${key}` : key;
    const a = saved?.[key];
    const b = draft?.[key];
    if (isPlainObject(a) && isPlainObject(b)) {
      out.push(...changedFields(a, b, path));
    } else if (JSON.stringify(a) !== JSON.stringify(b)) {
      out.push(path);
    }
  }
  return out.sort();
}

function isPlainObject(value) {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

/**
 * Check a draft against the published schema.
 *
 * @returns {{path: string, code: string, message: string}[]} — empty when the
 *   draft is acceptable. Codes match the store's own.
 */
export function validateDraft(schemaFields, draft, prefix = "") {
  const problems = [];
  if (!isPlainObject(draft)) {
    return [
      { path: prefix || "settings", code: "settings.type_mismatch", message: "must be an object" },
    ];
  }
  for (const [name, spec] of Object.entries(schemaFields ?? {})) {
    const path = prefix ? `${prefix}.${name}` : name;
    if (!Object.prototype.hasOwnProperty.call(draft, name)) {
      problems.push({
        path,
        code: "settings.field_missing",
        message: `${path} is required`,
      });
      continue;
    }
    problems.push(...checkField(path, spec, draft[name]));
  }
  for (const key of Object.keys(draft)) {
    if (!Object.prototype.hasOwnProperty.call(schemaFields ?? {}, key)) {
      const path = prefix ? `${prefix}.${key}` : key;
      problems.push({
        path,
        code: "settings.field_unknown",
        message: `${path} is not a settings field`,
      });
    }
  }
  return problems;
}

function checkField(path, spec, value) {
  switch (spec?.kind) {
    case "enum":
      if (typeof value !== "string") {
        return [{ path, code: "settings.type_mismatch", message: `${path} must be a string` }];
      }
      if (!(spec.values ?? []).includes(value)) {
        return [
          {
            path,
            code: "settings.value_not_allowed",
            message: `${path} must be one of ${(spec.values ?? []).join(", ")}`,
          },
        ];
      }
      return [];
    case "bool":
      return typeof value === "boolean"
        ? []
        : [{ path, code: "settings.type_mismatch", message: `${path} must be a boolean` }];
    case "const_bool":
      if (typeof value !== "boolean") {
        return [{ path, code: "settings.type_mismatch", message: `${path} must be a boolean` }];
      }
      return value === spec.value
        ? []
        : [
            {
              path,
              code: "settings.invariant_not_editable",
              message: `${path} is fixed at ${String(spec.value)}`,
            },
          ];
    case "int":
      if (typeof value !== "number" || !Number.isInteger(value)) {
        return [{ path, code: "settings.type_mismatch", message: `${path} must be an integer` }];
      }
      return value >= spec.min && value <= spec.max
        ? []
        : [
            {
              path,
              code: "settings.out_of_range",
              message: `${path} must be between ${spec.min} and ${spec.max}`,
            },
          ];
    case "object":
      return validateDraft(spec.fields, value, path);
    default:
      // An unrecognised spec is not silently accepted: the editor would be
      // rendering a control it does not understand.
      return [
        { path, code: "settings.spec_unknown", message: `${path} has an unrecognised spec` },
      ];
  }
}

/**
 * Keep only the fields the schema declares.
 *
 * A settings GET arrives inside a response envelope that carries extra keys
 * such as `snapshot_digest`. Sending those back would be refused as unknown
 * fields, so the draft is projected onto the schema rather than copied
 * wholesale. Declared fields that are absent stay absent, so a genuinely
 * missing field is still reported.
 */
export function projectToSchema(schemaFields, doc) {
  const out = {};
  if (!isPlainObject(doc)) return out;
  for (const [name, spec] of Object.entries(schemaFields ?? {})) {
    if (!Object.prototype.hasOwnProperty.call(doc, name)) continue;
    out[name] =
      spec?.kind === "object" ? projectToSchema(spec.fields, doc[name]) : doc[name];
  }
  return out;
}

/** Set a possibly nested path on a copy of `doc`. */
export function setPath(doc, path, value) {
  const parts = path.split(".");
  const next = structuredCloneish(doc);
  let cursor = next;
  for (const key of parts.slice(0, -1)) {
    if (!isPlainObject(cursor[key])) cursor[key] = {};
    cursor = cursor[key];
  }
  cursor[parts[parts.length - 1]] = value;
  return next;
}

function structuredCloneish(value) {
  return JSON.parse(JSON.stringify(value ?? {}));
}
