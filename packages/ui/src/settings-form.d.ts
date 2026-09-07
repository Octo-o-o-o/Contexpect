export interface SettingsProblem {
  path: string;
  code: string;
  message: string;
}
export declare function changedFields(
  saved: unknown,
  draft: unknown,
  prefix?: string,
): string[];
export declare function validateDraft(
  schemaFields: unknown,
  draft: unknown,
  prefix?: string,
): SettingsProblem[];
export declare function setPath<T>(doc: T, path: string, value: unknown): T;
export declare function projectToSchema(schemaFields: unknown, doc: unknown): Record<string, unknown>;
