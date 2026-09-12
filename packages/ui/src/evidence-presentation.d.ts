export declare function findingScope(finding: Record<string, unknown>, receipt: Record<string, unknown>): "receipt" | "linked-file" | "project";
export declare function findingSource(finding: Record<string, unknown>): "corpus" | "history" | "test" | "file";
export declare function displayTime(value: unknown, locale: string): string;
